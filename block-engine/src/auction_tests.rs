#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::Bundle;
    use tokio::sync::mpsc;
    use tokio::time::Duration;

    #[test]
    fn test_auction_window_ranking() {
        let bundles = vec![
            Bundle::new(vec![], 1000000, "searcher_a".to_string()), // 1 SOL
            Bundle::new(vec![], 2000000, "searcher_b".to_string()), // 2 SOL - should be first
            Bundle::new(vec![], 500000, "searcher_c".to_string()),  // 0.5 SOL
            Bundle::new(vec![], 1500000, "searcher_d".to_string()), // 1.5 SOL - should be second
        ];

        let window_id = 123;
        let max_bundles = 2;

        let winners = simulate_auction_with_bundles(window_id, bundles, max_bundles).unwrap();

        assert_eq!(winners.len(), 2);
        assert_eq!(winners[0].tip_lamports, 2000000); // Highest tip first
        assert_eq!(winners[1].tip_lamports, 1500000); // Second highest tip
        assert_eq!(winners[0].searcher_pubkey, "searcher_b");
        assert_eq!(winners[1].searcher_pubkey, "searcher_d");
    }

    #[test]
    fn test_auction_window_deterministic_tiebreaking() {
        // Create bundles with same tip to test deterministic tiebreaking
        let bundle_a = Bundle::new(vec![], 1000000, "searcher_a".to_string());
        let bundle_b = Bundle::new(vec![], 1000000, "searcher_b".to_string());
        
        let bundles = vec![bundle_b.clone(), bundle_a.clone()]; // Reverse order
        
        let winners = simulate_auction_with_bundles(1, bundles, 2).unwrap();
        
        // Should be sorted deterministically by bundle ID when tips are equal
        assert_eq!(winners.len(), 2);
        assert_eq!(winners[0].tip_lamports, 1000000);
        assert_eq!(winners[1].tip_lamports, 1000000);
        
        // The order should be deterministic based on bundle ID
        assert!(winners[0].id < winners[1].id);
    }

    #[tokio::test]
    async fn test_auction_window_timing() {
        let mut window = AuctionWindow::new(456, 50, 5); // 50ms window
        
        // Window should be open initially
        assert!(window.is_window_open());
        
        // Add a bundle
        let bundle = Bundle::new(vec![], 1000000, "test_searcher".to_string());
        assert!(window.try_add_bundle(bundle).unwrap());
        
        // Wait for window to close
        tokio::time::sleep(Duration::from_millis(60)).await;
        
        // Window should be closed now
        assert!(!window.is_window_open());
        
        // Cannot add bundles to closed window
        let late_bundle = Bundle::new(vec![], 2000000, "late_searcher".to_string());
        assert!(!window.try_add_bundle(late_bundle).unwrap());
    }

    #[test]
    fn test_auction_stats() {
        let bundles = vec![
            Bundle::new(vec![], 1000000, "searcher_1".to_string()),
            Bundle::new(vec![], 2000000, "searcher_2".to_string()),
            Bundle::new(vec![], 500000, "searcher_3".to_string()),
        ];

        let mut window = AuctionWindow::new(789, 200, 5);
        for bundle in bundles {
            window.try_add_bundle(bundle).unwrap();
        }

        let stats = window.get_auction_stats();
        assert_eq!(stats.total_bundles, 3);
        assert_eq!(stats.total_tip_value, 3500000);
        assert_eq!(stats.highest_tip, 2000000);
        assert_eq!(stats.lowest_tip, 500000);
        assert_eq!(stats.avg_tip, 1166666); // 3500000 / 3
    }
}
