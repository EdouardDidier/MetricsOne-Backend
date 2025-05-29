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
            "#[derive(serde::Serialize, serde::Deserialize)] \
             #[serde(rename_all = \"PascalCase\")]",
        )
        .field_attribute("InsertMeetingsRequest.Meeting.year", "#[serde(default)]")
        .message_attribute(
            "InsertMeetingsRequest.Meeting.Session",
            "#[derive(serde::Serialize, serde::Deserialize)] \
             #[serde(rename_all = \"PascalCase\")]",
        )
        .field_attribute(
            "InsertMeetingsRequest.Meeting.Session.start_date",
            "#[serde(with = \"crate::timestamp_format\")]",
        )
        .field_attribute(
            "InsertMeetingsRequest.Meeting.Session.end_date",
            "#[serde(with = \"crate::timestamp_format\")]",
        )
        .field_attribute(
            "InsertMeetingsRequest.Meeting.Session.gmt_offset",
            "#[sqlx(default)] \
             #[serde(with = \"crate::timestamp_format\"",
        )
        // .field_attribute(
        //     "InsertMeetingsRequest.Meeting.Session.start_date",
        //     "#[serde(deserialize_with = \"metrics_one_macros::deserialize_timestamp\")]"
        // )
        // .field_attribute("InsertMeetingsRequest.Meeting.Session", "#[sqlx(default)]")
        // .field_attribute("InsertMeetingsRequest.Meeting.start_date", "#[sqlx(default)]")
        // .extern_path(".google.protobuf.Any", "::prost_wkt_types::Any")
        // .extern_path(".google.protobuf.Timestamp", "::prost_wkt_types::Timestamp")
        // .extern_path(".google.protobuf.Value", "::prost_wkt_types::Value")
        .compile_protos(&["proto/insert.proto"], &["proto"])?;

    Ok(())
}
