use anyhow::{anyhow, Result};
use serde::Serialize;

use crate::config::BudgetPlanInput;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub(crate) enum BudgetStatus {
    Pass,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct BudgetPlanReport {
    pub status: BudgetStatus,
    pub max_connections: u32,
    pub reserved_connections: u32,
    pub safety_margin_connections: u32,
    pub available_connections: u32,
    pub current_total_connections: Option<u32>,
    pub requested_total_connections: u32,
    pub min_required_connections: u32,
    pub allocated_total_connections: u32,
    pub unused_connections: u32,
    pub over_budget_connections: u32,
    pub services: Vec<BudgetServiceAllocation>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct BudgetServiceAllocation {
    pub name: String,
    pub replicas: u32,
    pub priority: u32,
    pub current_pool_size: Option<u32>,
    pub min_pool_size: u32,
    pub max_pool_size: Option<u32>,
    pub recommended_pool_size: u32,
    pub desired_pool_size: u32,
    pub allocated_pool_size: u32,
    pub current_total_connections: Option<u32>,
    pub requested_total_connections: u32,
    pub allocated_total_connections: u32,
    pub pool_size_delta_from_current: Option<i32>,
    pub reduction_from_recommended: u32,
    pub capped_by_service_max: bool,
    pub meets_minimum: bool,
}

#[derive(Debug, Clone)]
struct AllocationState {
    service_index: usize,
    allocated_pool_size: u32,
}

pub(crate) fn build_budget_plan_report(input: BudgetPlanInput) -> Result<BudgetPlanReport> {
    validate_input(&input)?;

    let capacity = input
        .max_connections
        .saturating_sub(input.reserved_connections)
        .saturating_sub(input.safety_margin_connections);

    let mut allocations: Vec<BudgetServiceAllocation> = input
        .services
        .iter()
        .map(|service| {
            let desired_pool_size = service
                .max_pool_size
                .map(|max| service.recommended_pool_size.min(max))
                .unwrap_or(service.recommended_pool_size)
                .max(service.min_pool_size);
            BudgetServiceAllocation {
                name: service.name.trim().to_string(),
                replicas: service.replicas,
                priority: service.priority.unwrap_or(1),
                current_pool_size: service.current_pool_size,
                min_pool_size: service.min_pool_size,
                max_pool_size: service.max_pool_size,
                recommended_pool_size: service.recommended_pool_size,
                desired_pool_size,
                allocated_pool_size: service.min_pool_size,
                current_total_connections: service
                    .current_pool_size
                    .map(|pool| pool.saturating_mul(service.replicas)),
                requested_total_connections: desired_pool_size.saturating_mul(service.replicas),
                allocated_total_connections: service.min_pool_size.saturating_mul(service.replicas),
                pool_size_delta_from_current: None,
                reduction_from_recommended: service
                    .recommended_pool_size
                    .saturating_sub(desired_pool_size),
                capped_by_service_max: service
                    .max_pool_size
                    .map(|max| service.recommended_pool_size > max)
                    .unwrap_or(false),
                meets_minimum: true,
            }
        })
        .collect();

    let requested_total = allocations
        .iter()
        .map(|service| service.requested_total_connections)
        .sum::<u32>();
    let min_required = allocations
        .iter()
        .map(|service| service.allocated_total_connections)
        .sum::<u32>();
    let current_total = current_total_connections(&allocations);

    let mut warnings = Vec::new();
    if input.reserved_connections + input.safety_margin_connections >= input.max_connections {
        warnings.push(
            "reserved plus safety-margin connections leave no allocatable budget".to_string(),
        );
    }
    for service in &allocations {
        if service.capped_by_service_max {
            warnings.push(format!(
                "service '{}' recommendation {} was capped by max_pool_size {}",
                service.name,
                service.recommended_pool_size,
                service
                    .max_pool_size
                    .unwrap_or(service.recommended_pool_size)
            ));
        }
    }

    if min_required <= capacity {
        allocate_remaining_capacity(&mut allocations, capacity - min_required);
    } else {
        warnings.push(format!(
            "minimum service pools require {min_required} connections but only {capacity} are available"
        ));
    }

    for allocation in &mut allocations {
        allocation.allocated_total_connections = allocation
            .allocated_pool_size
            .saturating_mul(allocation.replicas);
        allocation.pool_size_delta_from_current = allocation
            .current_pool_size
            .map(|current| allocation.allocated_pool_size as i32 - current as i32);
        allocation.reduction_from_recommended = allocation
            .recommended_pool_size
            .saturating_sub(allocation.allocated_pool_size);
        allocation.meets_minimum = allocation.allocated_pool_size >= allocation.min_pool_size;
    }

    let allocated_total = allocations
        .iter()
        .map(|service| service.allocated_total_connections)
        .sum::<u32>();
    let status = if min_required > capacity {
        BudgetStatus::Critical
    } else if requested_total > capacity {
        BudgetStatus::Warning
    } else {
        BudgetStatus::Pass
    };
    if status == BudgetStatus::Warning {
        warnings.push(format!(
            "requested service pools need {requested_total} connections but only {capacity} are available"
        ));
    }

    Ok(BudgetPlanReport {
        status,
        max_connections: input.max_connections,
        reserved_connections: input.reserved_connections,
        safety_margin_connections: input.safety_margin_connections,
        available_connections: capacity,
        current_total_connections: current_total,
        requested_total_connections: requested_total,
        min_required_connections: min_required,
        allocated_total_connections: allocated_total,
        unused_connections: capacity.saturating_sub(allocated_total),
        over_budget_connections: requested_total.saturating_sub(capacity),
        services: allocations,
        warnings,
    })
}

fn validate_input(input: &BudgetPlanInput) -> Result<()> {
    if input.max_connections == 0 {
        return Err(anyhow!("max_connections must be greater than 0"));
    }
    if input.services.is_empty() {
        return Err(anyhow!("budget plan contains no services"));
    }

    for service in &input.services {
        let name = service.name.trim();
        if name.is_empty() {
            return Err(anyhow!("service names must not be empty"));
        }
        if service.replicas == 0 {
            return Err(anyhow!("service '{name}' replicas must be greater than 0"));
        }
        if service.min_pool_size == 0 {
            return Err(anyhow!(
                "service '{name}' min_pool_size must be greater than 0"
            ));
        }
        if service.recommended_pool_size == 0 {
            return Err(anyhow!(
                "service '{name}' recommended_pool_size must be greater than 0"
            ));
        }
        if service.min_pool_size > service.recommended_pool_size {
            return Err(anyhow!(
                "service '{name}' min_pool_size must be <= recommended_pool_size"
            ));
        }
        if let Some(max_pool_size) = service.max_pool_size {
            if max_pool_size < service.min_pool_size {
                return Err(anyhow!(
                    "service '{name}' max_pool_size must be >= min_pool_size"
                ));
            }
        }
    }

    Ok(())
}

fn allocate_remaining_capacity(allocations: &mut [BudgetServiceAllocation], mut remaining: u32) {
    let mut states: Vec<AllocationState> = allocations
        .iter()
        .enumerate()
        .map(|(service_index, service)| AllocationState {
            service_index,
            allocated_pool_size: service.allocated_pool_size,
        })
        .collect();

    while let Some(index) = next_allocation(&states, allocations, remaining) {
        let service_index = states[index].service_index;
        states[index].allocated_pool_size += 1;
        allocations[service_index].allocated_pool_size += 1;
        remaining = remaining.saturating_sub(allocations[service_index].replicas);
    }
}

fn next_allocation(
    states: &[AllocationState],
    allocations: &[BudgetServiceAllocation],
    remaining: u32,
) -> Option<usize> {
    states
        .iter()
        .enumerate()
        .filter(|(_, state)| {
            let service = &allocations[state.service_index];
            state.allocated_pool_size < service.desired_pool_size && service.replicas <= remaining
        })
        .max_by(|(_, left), (_, right)| {
            let left_service = &allocations[left.service_index];
            let right_service = &allocations[right.service_index];
            let left_score = allocation_score(left, left_service);
            let right_score = allocation_score(right, right_service);
            left_score
                .cmp(&right_score)
                .then_with(|| right_service.replicas.cmp(&left_service.replicas))
                .then_with(|| right_service.name.cmp(&left_service.name))
        })
        .map(|(index, _)| index)
}

fn allocation_score(state: &AllocationState, service: &BudgetServiceAllocation) -> u128 {
    let granted_extra = state
        .allocated_pool_size
        .saturating_sub(service.min_pool_size)
        + 1;
    u128::from(service.priority) * u128::from(service.replicas) * 1_000_000
        / u128::from(granted_extra)
}

fn current_total_connections(allocations: &[BudgetServiceAllocation]) -> Option<u32> {
    let mut total = 0u32;
    for service in allocations {
        total = total.checked_add(service.current_total_connections?)?;
    }
    Some(total)
}

#[cfg(test)]
mod tests {
    use crate::config::{BudgetPlanInput, BudgetServiceInput};

    use super::*;

    fn service(
        name: &str,
        replicas: u32,
        current: u32,
        recommended: u32,
        priority: u32,
    ) -> BudgetServiceInput {
        BudgetServiceInput {
            name: name.to_string(),
            replicas,
            current_pool_size: Some(current),
            min_pool_size: 2,
            max_pool_size: None,
            recommended_pool_size: recommended,
            priority: Some(priority),
        }
    }

    #[test]
    fn budget_plan_passes_when_recommendations_fit() {
        let report = build_budget_plan_report(BudgetPlanInput {
            max_connections: 100,
            reserved_connections: 10,
            safety_margin_connections: 5,
            services: vec![
                service("checkout", 3, 6, 8, 3),
                service("billing", 2, 4, 5, 1),
            ],
        })
        .expect("budget should plan");

        assert_eq!(report.status, BudgetStatus::Pass);
        assert_eq!(report.available_connections, 85);
        assert_eq!(report.requested_total_connections, 34);
        assert_eq!(report.allocated_total_connections, 34);
        assert_eq!(report.current_total_connections, Some(26));
        assert!(report.unused_connections > 0);
    }

    #[test]
    fn budget_plan_reduces_low_priority_services_when_over_budget() {
        let report = build_budget_plan_report(BudgetPlanInput {
            max_connections: 48,
            reserved_connections: 6,
            safety_margin_connections: 0,
            services: vec![
                service("checkout", 4, 6, 8, 5),
                service("billing", 3, 5, 8, 1),
            ],
        })
        .expect("budget should plan");

        assert_eq!(report.status, BudgetStatus::Warning);
        assert_eq!(report.available_connections, 42);
        assert!(report.requested_total_connections > report.available_connections);
        assert!(report.allocated_total_connections <= report.available_connections);
        let checkout = report
            .services
            .iter()
            .find(|service| service.name == "checkout")
            .expect("checkout allocation should exist");
        let billing = report
            .services
            .iter()
            .find(|service| service.name == "billing")
            .expect("billing allocation should exist");
        assert!(checkout.allocated_pool_size >= billing.allocated_pool_size);
        assert!(billing.reduction_from_recommended > 0);
    }

    #[test]
    fn budget_plan_reports_critical_when_minimums_do_not_fit() {
        let report = build_budget_plan_report(BudgetPlanInput {
            max_connections: 10,
            reserved_connections: 2,
            safety_margin_connections: 0,
            services: vec![
                service("checkout", 3, 4, 6, 1),
                service("billing", 3, 4, 6, 1),
            ],
        })
        .expect("budget should produce critical report");

        assert_eq!(report.status, BudgetStatus::Critical);
        assert!(report.min_required_connections > report.available_connections);
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("minimum service pools")));
    }

    #[test]
    fn budget_plan_validates_inputs_and_caps_service_max() {
        let err = build_budget_plan_report(BudgetPlanInput {
            max_connections: 0,
            reserved_connections: 0,
            safety_margin_connections: 0,
            services: vec![service("checkout", 1, 1, 1, 1)],
        })
        .expect_err("zero max connections should fail");
        assert!(err.to_string().contains("max_connections"));

        let mut capped = service("checkout", 2, 6, 10, 1);
        capped.max_pool_size = Some(7);
        let report = build_budget_plan_report(BudgetPlanInput {
            max_connections: 100,
            reserved_connections: 0,
            safety_margin_connections: 0,
            services: vec![capped],
        })
        .expect("capped service should plan");
        assert!(report.services[0].capped_by_service_max);
        assert_eq!(report.services[0].desired_pool_size, 7);

        let invalid = BudgetPlanInput {
            max_connections: 100,
            reserved_connections: 0,
            safety_margin_connections: 0,
            services: vec![BudgetServiceInput {
                name: " ".to_string(),
                replicas: 1,
                current_pool_size: None,
                min_pool_size: 1,
                max_pool_size: None,
                recommended_pool_size: 1,
                priority: None,
            }],
        };
        assert!(build_budget_plan_report(invalid).is_err());
    }

    #[test]
    fn budget_plan_covers_remaining_validation_errors_and_zero_capacity_warning() {
        let no_services = build_budget_plan_report(BudgetPlanInput {
            max_connections: 100,
            reserved_connections: 0,
            safety_margin_connections: 0,
            services: Vec::new(),
        })
        .expect_err("empty service list should fail");
        assert!(no_services.to_string().contains("contains no services"));

        let mut invalid = service("checkout", 0, 1, 2, 1);
        let err = build_budget_plan_report(BudgetPlanInput {
            max_connections: 100,
            reserved_connections: 0,
            safety_margin_connections: 0,
            services: vec![invalid.clone()],
        })
        .expect_err("zero replicas should fail");
        assert!(err.to_string().contains("replicas"));

        invalid.replicas = 1;
        invalid.min_pool_size = 0;
        let err = build_budget_plan_report(BudgetPlanInput {
            max_connections: 100,
            reserved_connections: 0,
            safety_margin_connections: 0,
            services: vec![invalid.clone()],
        })
        .expect_err("zero minimum should fail");
        assert!(err.to_string().contains("min_pool_size"));

        invalid.min_pool_size = 1;
        invalid.recommended_pool_size = 0;
        let err = build_budget_plan_report(BudgetPlanInput {
            max_connections: 100,
            reserved_connections: 0,
            safety_margin_connections: 0,
            services: vec![invalid.clone()],
        })
        .expect_err("zero recommendation should fail");
        assert!(err.to_string().contains("recommended_pool_size"));

        invalid.recommended_pool_size = 1;
        invalid.min_pool_size = 2;
        let err = build_budget_plan_report(BudgetPlanInput {
            max_connections: 100,
            reserved_connections: 0,
            safety_margin_connections: 0,
            services: vec![invalid.clone()],
        })
        .expect_err("minimum above recommendation should fail");
        assert!(err.to_string().contains("<= recommended_pool_size"));

        invalid.recommended_pool_size = 3;
        invalid.max_pool_size = Some(1);
        let err = build_budget_plan_report(BudgetPlanInput {
            max_connections: 100,
            reserved_connections: 0,
            safety_margin_connections: 0,
            services: vec![invalid],
        })
        .expect_err("max below minimum should fail");
        assert!(err.to_string().contains("max_pool_size"));

        let report = build_budget_plan_report(BudgetPlanInput {
            max_connections: 10,
            reserved_connections: 8,
            safety_margin_connections: 2,
            services: vec![service("checkout", 1, 1, 2, 1)],
        })
        .expect("zero allocatable budget should still produce a report");
        assert_eq!(report.status, BudgetStatus::Critical);
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("no allocatable budget")));
    }
}
