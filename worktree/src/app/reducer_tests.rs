#[cfg(test)]
mod tests {
    use crate::app::intent::Intent;
    use crate::app::model::AppState;
    use crate::app::test_utils::scaffolding::ReducerTestKit;
    use crate::domain::repository::Worktree;

    #[tokio::test]
    async fn test_initialize_flow_happy_path() {
        // 1. Setup
        let kit = ReducerTestKit::new();
        
        // 2. Act
        kit.reducer.handle(Intent::Initialize { 
            url: None, 
            name: Some("my-project".into()), 
            warp: false 
        }).await.unwrap();

        // 3. Assert (State Transitions)
        // Expect: Initializing -> Initialized
        // Note: Initializing state is rendered synchronously before the async block.
        // Initialized state is rendered after success.
        
        let states = kit.view.states.lock().unwrap();
        assert_eq!(states.len(), 2, "Expected 2 key state transitions");
        
        // Check first state
        match &states[0] {
            AppState::Initializing { project_name } => assert_eq!(project_name, "my-project"),
            _ => panic!("Expected Initializing state first, got {:?}", states[0]),
        }

        // Check second state
        match &states[1] {
            AppState::Initialized { project_name } => assert_eq!(project_name, "my-project"),
            _ => panic!("Expected Initialized state second, got {:?}", states[1]),
        }
    }
    
    #[tokio::test]
    async fn test_list_worktrees_empty() {
        let kit = ReducerTestKit::new();
        
        kit.reducer.handle(Intent::ListWorktrees).await.unwrap();
        
        // Assert: Banner -> Welcome Screen (implied by empty list) -> Listing Table
        // The View implementation for ListWorktrees:
        // 1. render_banner()
        // 2. if empty: render(Welcome)
        // 3. render_listing_table()
        
        let banners = *kit.view.banners.lock().unwrap();
        assert_eq!(banners, 1, "Should have rendered banner");
        
        let states = kit.view.states.lock().unwrap();
        assert!(!states.is_empty(), "Should have rendered at least Welcome state");
        match &states[0] {
            AppState::Welcome => {},
            _ => panic!("Expected Welcome state for empty worktree list, got {:?}", states[0]),
        }
        
        let listings = kit.view.listings.lock().unwrap();
        assert_eq!(listings.len(), 1, "Should have rendered listing table");
        assert!(listings[0].is_empty(), "Listing should be empty");
    }
}
