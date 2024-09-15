use zenoh::bytes::Encoding;

enum SampleViewerPage {
    Raw,
    Parse,
}

pub struct SampleViewer {
    select_page: SampleViewerPage,
    key: String,
    encoding: Encoding,
}
