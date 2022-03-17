use cap_async_std::net::{Ipv4Addr, SocketAddr};
use tide::Result;

#[async_std::main]
async fn main() -> Result<()> {
    drawbridge::new()
        .listen(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 8000)))
        .await?;
    Ok(())
}
