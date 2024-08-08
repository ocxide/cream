/// config for providing repositories, EventBusPort, etc.
pub mod context;
/// ports & sockets for emitting & recieving events
pub mod event_bus;
/// define events, handlers & config the router
pub mod events;
/// listen for events and dispatch to handlers
pub mod router_bus;
