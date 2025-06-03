use crate::{
    domain::{MicroserviceInfo, ServiceStatus},
    utils::errors::Result,
};
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug)]
pub struct MicroserviceRegistry {
    services: Arc<DashMap<String, MicroserviceInfo>>,
}

impl MicroserviceRegistry {
    pub fn new() -> Self {
        Self {
            services: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_service(&self, service: MicroserviceInfo) -> Result<()> {
        tracing::info!("Registering microservice: {}", service.service_id);
        self.services.insert(service.service_id.clone(), service);
        Ok(())
    }

    pub async fn get_service(&self, service_id: &str) -> Result<Option<MicroserviceInfo>> {
        Ok(self.services.get(service_id).map(|entry| entry.clone()))
    }

    pub async fn get_services_by_ids(
        &self,
        service_ids: &[String],
    ) -> Result<Vec<MicroserviceInfo>> {
        let mut services = Vec::new();
        for service_id in service_ids {
            if let Some(service) = self.services.get(service_id) {
                if service.is_available() {
                    services.push(service.clone());
                }
            }
        }
        Ok(services)
    }

    pub async fn get_all_available_services(&self) -> Result<Vec<MicroserviceInfo>> {
        Ok(self
            .services
            .iter()
            .filter(|entry| entry.is_available())
            .map(|entry| entry.clone())
            .collect())
    }

    pub async fn update_service_status(
        &self,
        service_id: &str,
        status: ServiceStatus,
    ) -> Result<()> {
        if let Some(mut service) = self.services.get_mut(service_id) {
            service.update_status(status);
            tracing::debug!(
                "Updated service {} status to {:?}",
                service_id,
                service.status
            );
        }
        Ok(())
    }

    pub async fn unregister_service(&self, service_id: &str) -> Result<()> {
        self.services.remove(service_id);
        tracing::info!("Unregistered microservice: {}", service_id);
        Ok(())
    }

    pub async fn list_all_services(&self) -> Result<Vec<MicroserviceInfo>> {
        Ok(self.services.iter().map(|entry| entry.clone()).collect())
    }

    pub async fn get_service_count(&self) -> usize {
        self.services.len()
    }
}

impl Default for MicroserviceRegistry {
    fn default() -> Self {
        Self::new()
    }
}
