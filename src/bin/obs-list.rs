use obws::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::connect("localhost", 4455, Some("123123")).await?;

    println!("\n=== OBS Scenes and Filters ===\n");

    // Get all scenes
    let scenes_response = client.scenes().list().await?;

    if let Some(current) = &scenes_response.current_program_scene {
        println!("Current scene: {}\n", current.name);
    }

    for scene in &scenes_response.scenes {
        println!("Scene: {} (index: {})", scene.id.name, scene.index);

        // List sources in this scene
        let scene_id = obws::requests::scenes::SceneId::Name(&scene.id.name);
        match client.scene_items().list(scene_id).await {
            Ok(items) => {
                for item in &items {
                    println!("  └─ Source: {} (id: {})", item.source_name, item.id);

                    // List filters for this source
                    let source_id = obws::requests::sources::SourceId::Name(&item.source_name);
                    match client.filters().list(source_id).await {
                        Ok(filters) => {
                            for filter in filters {
                                println!("      └─ Filter: {} [{}] (enabled: {})",
                                    filter.name, filter.kind, filter.enabled);
                            }
                        }
                        Err(_) => {
                            // Source doesn't support filters
                        }
                    }
                }
            }
            Err(e) => {
                println!("  Error listing scene items: {}", e);
            }
        }
        println!();
    }

    Ok(())
}
