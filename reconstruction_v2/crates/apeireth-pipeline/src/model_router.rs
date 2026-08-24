//! Model router — semantic-aware route decision.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelRoute {
    pub provider: String,
    pub model: String,
    pub weight: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RouteDecision {
    Use(ModelRoute),
    Reject,
}

#[derive(Debug, Default)]
pub struct ModelRouter {
    routes: Vec<ModelRoute>,
}

impl ModelRouter {
    pub fn new() -> Self { Self::default() }
    pub fn add_route(&mut self, route: ModelRoute) { self.routes.push(route); }
    pub fn route(&self) -> RouteDecision {
        match self.routes.first() {
            Some(r) => RouteDecision::Use(r.clone()),
            None => RouteDecision::Reject,
        }
    }
    pub fn routes(&self) -> &[ModelRoute] { &self.routes }
    pub fn len(&self) -> usize { self.routes.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_router_rejects() {
        let r = ModelRouter::new();
        assert!(matches!(r.route(), RouteDecision::Reject));
    }

    #[test]
    fn router_returns_first() {
        let mut r = ModelRouter::new();
        r.add_route(ModelRoute { provider: "p".into(), model: "m1".into(), weight: 1.0 });
        r.add_route(ModelRoute { provider: "p".into(), model: "m2".into(), weight: 0.5 });
        match r.route() {
            RouteDecision::Use(route) => assert_eq!(route.model, "m1"),
            _ => panic!(),
        }
    }
}
