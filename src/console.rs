use std::fmt::Display;

#[derive(PartialEq, Eq)]
pub enum Console {
    Old3DSLatest,
    New3DSLatest,
    Old3DSOld,
    New3DSOld,
}

impl Console {
    fn encoded_id1(&self) -> &'static str {
        match self {
            Console::Old3DSLatest => {
                "fffffffa119907488546696508a10122054b984768465946c0aa171c4346034ca047b84700900a0871a0050899ce0408730064006d00630000900a0862003900"
            }
            Console::New3DSLatest => {
                "fffffffa119907488546696508a10122054b984768465946c0aa171c4346034ca047b84700900a0871a005085dce0408730064006d00630000900a0862003900"
            }
            Console::Old3DSOld => {
                "fffffffa119907488546696508a10122054b984768465946c0aa171c4346034ca047b84700900a08499e050899cc0408730064006d00630000900a0862003900"
            }
            Console::New3DSOld => {
                "fffffffa119907488546696508a10122054b984768465946c0aa171c4346034ca047b84700900a08459e050881cc0408730064006d00630000900a0862003900"
            }
        }
    }
    pub fn encoded_id1_readable(&self) -> String {
        let id1 = self.encoded_id1();
        let bytes = hex::decode(id1).expect("Failed to decode ID1 hex string");
        String::from_utf16(
            &bytes
                .chunks_exact(2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .collect::<Vec<u16>>(),
        )
        .expect("Failed to convert ID1 bytes to string")
    }

    pub fn new_from_encoded_id1(encoded_id1: &str) -> Option<Self> {
        [
            Console::Old3DSLatest,
            Console::New3DSLatest,
            Console::Old3DSOld,
            Console::New3DSOld,
        ]
        .into_iter()
        .find(|console| console.encoded_id1() == encoded_id1)
    }

    pub fn model(&self) -> &'static str {
        match self {
            Console::Old3DSLatest | Console::Old3DSOld => "Old 3DS/2DS",
            Console::New3DSLatest | Console::New3DSOld => "New 3DS/2DS",
        }
    }

    pub fn version(&self) -> &'static str {
        match self {
            Console::Old3DSLatest | Console::New3DSLatest => "11.8.0 to 11.17.0",
            Console::Old3DSOld | Console::New3DSOld => "11.4.0 to 11.7.0",
        }
    }
}

impl Display for Console {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, {}", self.model(), self.version())
    }
}
