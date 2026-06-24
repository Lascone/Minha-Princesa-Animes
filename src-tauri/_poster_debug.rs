use minha_princesa_animes_lib::sushi::client::SushiClient;
use minha_princesa_animes_lib::sushi::browse_catalog;
use minha_princesa_animes_lib::models::CatalogType;

#[tokio::main]
async fn main() {
    let client = SushiClient::new().unwrap();
    let page = browse_catalog(&client, CatalogType::Animes, 1, None).await.unwrap();
    for item in page.items.iter().take(5) {
        println!("{} | poster={:?}", item.title, item.poster);
    }
}
