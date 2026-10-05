use gdp_axum_basic::app;
use gdp_axum_basic::db::{Db, Plan, Role};

#[tokio::main]
async fn main() {
    let db = Db::default();
    db.seed(
        &[
            ("alice", "acme", Role::Owner),
            ("bob", "acme", Role::Viewer),
        ],
        &[("acme", Plan::Pro)],
    )
    .await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on http://{}", listener.local_addr().unwrap());
    axum::serve(listener, app(db)).await.unwrap();
}
