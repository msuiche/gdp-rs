// Same for async scopes.
#[path = "fixture/mod.rs"] mod fixture;
use fixture::*;

async fn handler() {
    let _stash = gdp::name2_async(UserId(1), ProjectId(1), async |user, project| admin::check(&user, &project)).await;
}

fn main() {
    let _ = handler();
}
