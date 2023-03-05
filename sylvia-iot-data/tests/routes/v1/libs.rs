use mongodb::bson::Document;
use sql_builder::SqlBuilder;
use sqlx;
use tokio::runtime::Runtime;

use crate::TestState;

pub fn clear_all_data(runtime: &Runtime, state: &TestState) -> () {
    const APP_DLDATA_NAME: &'static str = "applicationDlData";
    const APP_DLDATA_NAME2: &'static str = "application_dldata";
    const APP_ULDATA_NAME: &'static str = "applicationUlData";
    const APP_ULDATA_NAME2: &'static str = "application_uldata";
    const NET_DLDATA_NAME: &'static str = "networkDlData";
    const NET_DLDATA_NAME2: &'static str = "network_dldata";
    const NET_ULDATA_NAME: &'static str = "networkUlData";
    const NET_ULDATA_NAME2: &'static str = "network_uldata";
    const COREMGR_OPDATA_NAME: &'static str = "coremgrOpData";
    const COREMGR_OPDATA_NAME2: &'static str = "coremgr_opdata";

    if let Some(model) = state.mongodb.as_ref() {
        runtime.block_on(async {
            struct Doc;

            let conn = model.get_connection();
            let _ = conn
                .collection::<Doc>(APP_DLDATA_NAME)
                .delete_many(Document::new(), None)
                .await;
            let _ = conn
                .collection::<Doc>(APP_ULDATA_NAME)
                .delete_many(Document::new(), None)
                .await;
            let _ = conn
                .collection::<Doc>(NET_DLDATA_NAME)
                .delete_many(Document::new(), None)
                .await;
            let _ = conn
                .collection::<Doc>(NET_ULDATA_NAME)
                .delete_many(Document::new(), None)
                .await;
            let _ = conn
                .collection::<Doc>(COREMGR_OPDATA_NAME)
                .delete_many(Document::new(), None)
                .await;
        });
    }
    if let Some(model) = state.sqlite.as_ref() {
        runtime.block_on(async {
            let conn = model.get_connection();
            let sql = SqlBuilder::delete_from(APP_DLDATA_NAME2).sql().unwrap();
            let _ = sqlx::query(sql.as_str()).execute(conn).await;
            let sql = SqlBuilder::delete_from(APP_ULDATA_NAME2).sql().unwrap();
            let _ = sqlx::query(sql.as_str()).execute(conn).await;
            let sql = SqlBuilder::delete_from(NET_DLDATA_NAME2).sql().unwrap();
            let _ = sqlx::query(sql.as_str()).execute(conn).await;
            let sql = SqlBuilder::delete_from(NET_ULDATA_NAME2).sql().unwrap();
            let _ = sqlx::query(sql.as_str()).execute(conn).await;
            let sql = SqlBuilder::delete_from(COREMGR_OPDATA_NAME2).sql().unwrap();
            let _ = sqlx::query(sql.as_str()).execute(conn).await;
        });
    }
}
