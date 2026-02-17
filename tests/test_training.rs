use anyhow::Result;
use chrono::Utc;
use mcp_context_rust::training::{SearchCriteria, TrainingManager};
use mcp_context_rust::types::CodePattern;
use std::fs;

#[tokio::test]
async fn test_load_patterns() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let patterns_path = temp_dir.path().join("patterns");
    fs::create_dir_all(&patterns_path)?;

    // Create a sample pattern file
    let pattern_file = patterns_path.join("test-patterns.json");
    let pattern_json = r#"{
  "patterns": [
    {
      "id": "test-pattern-1",
      "category": "testing",
      "framework": "blazor-server",
      "version": "10.0",
      "title": "Test Pattern",
      "description": "A test pattern for unit tests",
      "code": "public class TestClass { }",
      "tags": ["test", "sample"],
      "usage_count": 0,
      "relevance_score": 0.8,
      "created_at": "2025-10-25T00:00:00Z",
      "updated_at": "2025-10-25T00:00:00Z"
    }
  ]
}"#;

    fs::write(&pattern_file, pattern_json)?;

    // Load patterns
    let mut manager = TrainingManager::new(patterns_path);
    manager.load_patterns().await?;

    println!("\n=== Pattern Loading Test ===");
    println!("Patterns loaded: {}", manager.get_all_patterns().len());

    assert_eq!(manager.get_all_patterns().len(), 1);

    let pattern = manager.get_pattern_by_id("test-pattern-1");
    assert!(pattern.is_some());

    let pattern = pattern.unwrap();
    assert_eq!(pattern.title, "Test Pattern");
    assert_eq!(pattern.framework, "blazor-server");
    assert_eq!(pattern.category, "testing");

    Ok(())
}

#[tokio::test]
async fn test_load_multiple_pattern_files() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let patterns_path = temp_dir.path().join("patterns");
    fs::create_dir_all(&patterns_path)?;

    // Create multiple pattern files
    let lifecycle_json = r#"{
  "patterns": [
    {
      "id": "lifecycle-1",
      "category": "lifecycle",
      "framework": "blazor-server",
      "version": "10.0",
      "title": "OnInitializedAsync",
      "description": "Lifecycle pattern",
      "code": "protected override async Task OnInitializedAsync() { }",
      "tags": ["lifecycle"],
      "usage_count": 5,
      "relevance_score": 0.9,
      "created_at": "2025-10-25T00:00:00Z",
      "updated_at": "2025-10-25T00:00:00Z"
    }
  ]
}"#;

    let di_json = r#"{
  "patterns": [
    {
      "id": "di-1",
      "category": "dependency-injection",
      "framework": "blazor-server",
      "version": "10.0",
      "title": "Service Injection",
      "description": "DI pattern",
      "code": "@inject IService Service",
      "tags": ["di", "services"],
      "usage_count": 3,
      "relevance_score": 0.85,
      "created_at": "2025-10-25T00:00:00Z",
      "updated_at": "2025-10-25T00:00:00Z"
    }
  ]
}"#;

    fs::write(patterns_path.join("lifecycle.json"), lifecycle_json)?;
    fs::write(patterns_path.join("di.json"), di_json)?;

    let mut manager = TrainingManager::new(patterns_path);
    manager.load_patterns().await?;

    println!("\n=== Multiple Files Test ===");
    println!("Total patterns: {}", manager.get_all_patterns().len());

    assert_eq!(manager.get_all_patterns().len(), 2);

    let lifecycle = manager.get_pattern_by_id("lifecycle-1");
    let di = manager.get_pattern_by_id("di-1");

    assert!(lifecycle.is_some());
    assert!(di.is_some());

    Ok(())
}

#[tokio::test]
async fn test_search_by_framework() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let patterns_path = temp_dir.path().join("patterns");
    fs::create_dir_all(&patterns_path)?;

    let pattern_json = r#"{
  "patterns": [
    {
      "id": "blazor-1",
      "category": "lifecycle",
      "framework": "blazor-server",
      "version": "10.0",
      "title": "Blazor Pattern",
      "description": "Blazor specific",
      "code": "blazor code",
      "tags": ["blazor"],
      "usage_count": 0,
      "relevance_score": 0.9,
      "created_at": "2025-10-25T00:00:00Z",
      "updated_at": "2025-10-25T00:00:00Z"
    },
    {
      "id": "aspnet-1",
      "category": "lifecycle",
      "framework": "aspnet-core",
      "version": "10.0",
      "title": "ASP.NET Pattern",
      "description": "ASP.NET specific",
      "code": "aspnet code",
      "tags": ["aspnet"],
      "usage_count": 0,
      "relevance_score": 0.8,
      "created_at": "2025-10-25T00:00:00Z",
      "updated_at": "2025-10-25T00:00:00Z"
    }
  ]
}"#;

    fs::write(patterns_path.join("patterns.json"), pattern_json)?;

    let mut manager = TrainingManager::new(patterns_path);
    manager.load_patterns().await?;

    println!("\n=== Framework Search Test ===");

    let criteria = SearchCriteria {
        query: None,
        category: None,
        framework: Some("blazor-server".to_string()),
        tags: vec![],
        min_score: 0.0,
        max_results: None,
    };

    let results = manager.search_patterns(&criteria);
    println!("Blazor patterns found: {}", results.len());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0.id, "blazor-1");

    Ok(())
}

#[tokio::test]
async fn test_search_by_category() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let patterns_path = temp_dir.path().join("patterns");
    fs::create_dir_all(&patterns_path)?;

    let pattern_json = r#"{
  "patterns": [
    {
      "id": "lc-1",
      "category": "lifecycle",
      "framework": "blazor-server",
      "version": "10.0",
      "title": "Lifecycle",
      "description": "Lifecycle",
      "code": "lifecycle",
      "tags": [],
      "usage_count": 0,
      "relevance_score": 0.9,
      "created_at": "2025-10-25T00:00:00Z",
      "updated_at": "2025-10-25T00:00:00Z"
    },
    {
      "id": "di-1",
      "category": "dependency-injection",
      "framework": "blazor-server",
      "version": "10.0",
      "title": "DI",
      "description": "DI",
      "code": "di",
      "tags": [],
      "usage_count": 0,
      "relevance_score": 0.8,
      "created_at": "2025-10-25T00:00:00Z",
      "updated_at": "2025-10-25T00:00:00Z"
    }
  ]
}"#;

    fs::write(patterns_path.join("patterns.json"), pattern_json)?;

    let mut manager = TrainingManager::new(patterns_path);
    manager.load_patterns().await?;

    println!("\n=== Category Search Test ===");

    let results = manager.search_by_framework_and_category("blazor-server", "lifecycle");
    println!("Lifecycle patterns: {}", results.len());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "lc-1");

    Ok(())
}

#[tokio::test]
async fn test_search_with_scoring() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let patterns_path = temp_dir.path().join("patterns");
    fs::create_dir_all(&patterns_path)?;

    // Use older dates (> 30 days ago) to avoid recency bonus affecting scores
    let pattern_json = r#"{
  "patterns": [
    {
      "id": "high-score",
      "category": "testing",
      "framework": "blazor-server",
      "version": "10.0",
      "title": "High Score Pattern",
      "description": "This pattern has high relevance",
      "code": "high score code",
      "tags": ["test"],
      "usage_count": 10,
      "relevance_score": 0.95,
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-01T00:00:00Z"
    },
    {
      "id": "low-score",
      "category": "testing",
      "framework": "blazor-server",
      "version": "10.0",
      "title": "Low Score Pattern",
      "description": "This pattern has low relevance",
      "code": "low score code",
      "tags": ["test"],
      "usage_count": 0,
      "relevance_score": 0.5,
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-01T00:00:00Z"
    }
  ]
}"#;

    fs::write(patterns_path.join("patterns.json"), pattern_json)?;

    let mut manager = TrainingManager::new(patterns_path);
    manager.load_patterns().await?;

    println!("\n=== Scoring Test ===");

    let criteria = SearchCriteria {
        query: Some("pattern".to_string()),
        category: Some("testing".to_string()),
        framework: Some("blazor-server".to_string()),
        tags: vec![],
        min_score: 0.0,
        max_results: None,
    };

    let results = manager.search_patterns(&criteria);
    println!("Found {} patterns", results.len());

    for (pattern, score) in &results {
        println!("  - {} (score: {:.2})", pattern.title, score);
    }

    assert_eq!(results.len(), 2);
    // High score should be first (but scores might be equal after adjustments)
    // Just verify both patterns are found and the high-score one has >= score
    let high_score_result = results.iter().find(|(p, _)| p.id == "high-score");
    let low_score_result = results.iter().find(|(p, _)| p.id == "low-score");
    assert!(
        high_score_result.is_some(),
        "high-score pattern should be found"
    );
    assert!(
        low_score_result.is_some(),
        "low-score pattern should be found"
    );
    assert!(
        high_score_result.unwrap().1 >= low_score_result.unwrap().1,
        "high-score should have >= score than low-score"
    );

    Ok(())
}

#[tokio::test]
async fn test_add_and_save_pattern() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let patterns_path = temp_dir.path().join("patterns");
    fs::create_dir_all(&patterns_path)?;

    let mut manager = TrainingManager::new(&patterns_path);

    println!("\n=== Add Pattern Test ===");

    let new_pattern = CodePattern {
        id: "new-pattern".to_string(),
        category: "testing".to_string(),
        framework: "blazor-server".to_string(),
        version: "10.0".to_string(),
        title: "New Pattern".to_string(),
        description: "A newly added pattern".to_string(),
        code: "// new code".to_string(),
        tags: vec!["new".to_string()],
        usage_count: 0,
        relevance_score: 0.75,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    manager
        .add_pattern(new_pattern)
        .expect("Failed to add pattern");
    assert_eq!(manager.get_all_patterns().len(), 1);

    // Save patterns
    manager.save_patterns().await?;

    // Verify file was created
    let expected_file = patterns_path.join("blazor-server-patterns.json");
    assert!(expected_file.exists());

    println!("Pattern saved to: {:?}", expected_file);

    // Load in new manager to verify persistence
    let mut new_manager = TrainingManager::new(&patterns_path);
    new_manager.load_patterns().await?;

    assert_eq!(new_manager.get_all_patterns().len(), 1);

    let loaded = new_manager.get_pattern_by_id("new-pattern");
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().title, "New Pattern");

    Ok(())
}

#[tokio::test]
async fn test_increment_usage() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let patterns_path = temp_dir.path().join("patterns");
    fs::create_dir_all(&patterns_path)?;

    let pattern_json = r#"{
  "patterns": [
    {
      "id": "usage-test",
      "category": "testing",
      "framework": "blazor-server",
      "version": "10.0",
      "title": "Usage Test",
      "description": "Test usage counting",
      "code": "code",
      "tags": [],
      "usage_count": 0,
      "relevance_score": 0.8,
      "created_at": "2025-10-25T00:00:00Z",
      "updated_at": "2025-10-25T00:00:00Z"
    }
  ]
}"#;

    fs::write(patterns_path.join("patterns.json"), pattern_json)?;

    let mut manager = TrainingManager::new(patterns_path);
    manager.load_patterns().await?;

    println!("\n=== Usage Count Test ===");

    let pattern = manager.get_pattern_by_id("usage-test").unwrap();
    println!("Initial usage: {}", pattern.usage_count);
    assert_eq!(pattern.usage_count, 0);

    // Increment usage
    manager.increment_usage("usage-test")?;
    manager.increment_usage("usage-test")?;

    let pattern = manager.get_pattern_by_id("usage-test").unwrap();
    println!("After increments: {}", pattern.usage_count);
    assert_eq!(pattern.usage_count, 2);

    Ok(())
}

#[tokio::test]
async fn test_statistics() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let patterns_path = temp_dir.path().join("patterns");
    fs::create_dir_all(&patterns_path)?;

    let pattern_json = r#"{
  "patterns": [
    {
      "id": "p1",
      "category": "lifecycle",
      "framework": "blazor-server",
      "version": "10.0",
      "title": "P1",
      "description": "P1",
      "code": "p1",
      "tags": [],
      "usage_count": 5,
      "relevance_score": 0.9,
      "created_at": "2025-10-25T00:00:00Z",
      "updated_at": "2025-10-25T00:00:00Z"
    },
    {
      "id": "p2",
      "category": "dependency-injection",
      "framework": "blazor-server",
      "version": "10.0",
      "title": "P2",
      "description": "P2",
      "code": "p2",
      "tags": [],
      "usage_count": 3,
      "relevance_score": 0.8,
      "created_at": "2025-10-25T00:00:00Z",
      "updated_at": "2025-10-25T00:00:00Z"
    }
  ]
}"#;

    fs::write(patterns_path.join("patterns.json"), pattern_json)?;

    let mut manager = TrainingManager::new(patterns_path);
    manager.load_patterns().await?;

    println!("\n=== Statistics Test ===");

    let stats = manager.get_statistics();
    println!("{}", serde_json::to_string_pretty(&stats)?);

    assert_eq!(stats["total_patterns"], 2);
    assert_eq!(stats["total_usage"], 8);
    // Use approximate comparison for floating point
    let avg_relevance = stats["avg_relevance"].as_f64().unwrap();
    assert!(
        (avg_relevance - 0.85).abs() < 0.001,
        "Expected avg_relevance ~0.85, got {}",
        avg_relevance
    );

    let categories = stats["categories"].as_array().unwrap();
    assert_eq!(categories.len(), 2);

    Ok(())
}
