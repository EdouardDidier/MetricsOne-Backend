use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    tonic_build::compile_protos("./proto/fetch.proto")?;
    tonic_build::configure()
        .message_attribute(
            "InsertMeetingsRequest",
            "#[derive(serde::Deserialize)] \
             #[serde(rename_all = \"PascalCase\")]",
        )
        .message_attribute(
            "InsertMeetingsRequest.Meeting",
            "#[derive(serde::Serialize, serde::Deserialize, sqlx::FromRow, metrics_one_macros::SqlNames)] \
             #[serde(rename_all = \"PascalCase\")] \
             #[sql_names(table_name = \"meetings\")]",
        )
        .field_attribute("InsertMeetingsRequest.Meeting.year", "#[serde(default)]")
        .compile_protos(&["proto/insert.proto"], &["proto"])?;

    Ok(())
}
