use crate::consensus::ConsensusManager;
use crate::engine::QianjiEngine;
use crate::scheduler::identity::SchedulerAgentIdentity;
use crate::swarm::RemotePossessionBus;
use std::sync::Arc;

use super::{QianjiScheduler, SchedulerRuntimeServices};

impl QianjiScheduler {
    /// Creates a new scheduler for the given engine.
    #[must_use]
    pub fn new(engine: QianjiEngine) -> Self {
        Self::with_consensus_manager(engine, None)
    }

    /// Creates a new scheduler with optional distributed consensus manager.
    #[must_use]
    pub fn with_consensus_manager(
        engine: QianjiEngine,
        consensus_manager: Option<Arc<ConsensusManager>>,
    ) -> Self {
        let services = SchedulerRuntimeServices {
            consensus_manager,
            ..SchedulerRuntimeServices::default()
        };
        Self::with_runtime_services_config(engine, SchedulerAgentIdentity::from_env(), services)
    }

    /// Creates a scheduler with optional distributed consensus manager and explicit
    /// execution identity for role-aware swarm routing.
    #[must_use]
    pub fn with_consensus_manager_and_identity(
        engine: QianjiEngine,
        consensus_manager: Option<Arc<ConsensusManager>>,
        execution_identity: SchedulerAgentIdentity,
    ) -> Self {
        let services = SchedulerRuntimeServices {
            consensus_manager,
            ..SchedulerRuntimeServices::default()
        };
        Self::with_runtime_services_config(engine, execution_identity, services)
    }

    /// Creates a scheduler with full runtime services, including optional cross-cluster
    /// possession bus used for remote role execution.
    #[must_use]
    pub fn with_runtime_services(
        engine: QianjiEngine,
        consensus_manager: Option<Arc<ConsensusManager>>,
        remote_possession_bus: Option<Arc<RemotePossessionBus>>,
        cluster_id: Option<String>,
        execution_identity: SchedulerAgentIdentity,
    ) -> Self {
        let services = SchedulerRuntimeServices {
            consensus_manager,
            remote_possession_bus,
            cluster_id,
            ..SchedulerRuntimeServices::default()
        };
        Self::with_runtime_services_config(engine, execution_identity, services)
    }

    /// Creates a scheduler with an explicit runtime service bundle and policy.
    #[must_use]
    pub fn with_runtime_services_config(
        engine: QianjiEngine,
        execution_identity: SchedulerAgentIdentity,
        services: SchedulerRuntimeServices,
    ) -> Self {
        let cluster_id = services
            .cluster_id
            .or_else(|| std::env::var("CLUSTER_ID").ok())
            .unwrap_or_else(|| "local_cluster".to_string());
        Self {
            engine: Arc::new(tokio::sync::RwLock::new(engine)),
            max_total_steps: 1000,
            consensus_manager: services.consensus_manager,
            remote_possession_bus: services.remote_possession_bus,
            role_registry: services.role_registry,
            cluster_id,
            execution_identity,
            execution_policy: services.execution_policy,
            telemetry_emitter: services.telemetry_emitter,
        }
    }
}
