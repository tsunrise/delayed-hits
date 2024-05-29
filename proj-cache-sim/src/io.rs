use proj_models::{codec::Codec, RequestEvent};

pub fn load_events_file(path: &str) -> impl Iterator<Item = RequestEvent> {
    let reader = std::fs::File::open(path).unwrap();
    let reader = std::io::BufReader::new(reader);
    RequestEvent::repeat_read_till_end(reader).map(|r| r.unwrap())
}
