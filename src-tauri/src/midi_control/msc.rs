use super::ShowControlCommand;

pub const MSC_MANUFACTURER_ID: [u8; 3] = [0x7F, 0x7F, 0x02];
pub const MSC_GO: u8 = 0x01;
pub const MSC_STOP: u8 = 0x02;
pub const MSC_RESUME: u8 = 0x03;
pub const MSC_PAUSE: u8 = 0x07;
pub const MSC_RESET: u8 = 0x0D;
pub const MSC_GO_OFF: u8 = 0x10;

pub fn parse_msc(data: &[u8]) -> Option<ShowControlCommand> {
    if data.len() < 7 { return None; }
    if data[0] != 0xF0 { return None; }
    if data[1] != 0x7F { return None; }
    if data[3] != 0x02 { return None; }
    let cmd = data[4];
    match cmd {
        MSC_GO => Some(ShowControlCommand::Go),
        MSC_STOP => Some(ShowControlCommand::Stop),
        MSC_RESUME => Some(ShowControlCommand::Resume),
        MSC_PAUSE => Some(ShowControlCommand::Pause),
        MSC_RESET => Some(ShowControlCommand::Reset),
        MSC_GO_OFF => Some(ShowControlCommand::GoOff),
        0x0B if data.len() > 5 => {
            let fire = data[5];
            Some(ShowControlCommand::Fire(fire))
        }
        _ => None,
    }
}

pub fn build_msc(command: ShowControlCommand, device_id: u8) -> Vec<u8> {
    let mut msg = vec![0xF0, 0x7F, device_id, 0x02];
    match command {
        ShowControlCommand::Go => msg.push(MSC_GO),
        ShowControlCommand::Stop => msg.push(MSC_STOP),
        ShowControlCommand::Resume => msg.push(MSC_RESUME),
        ShowControlCommand::Pause => msg.push(MSC_PAUSE),
        ShowControlCommand::Reset => msg.push(MSC_RESET),
        ShowControlCommand::GoOff => msg.push(MSC_GO_OFF),
        ShowControlCommand::Fire(n) => { msg.push(0x0B); msg.push(n); }
        ShowControlCommand::GoCue(cue) => {
            msg.push(MSC_GO);
            msg.extend_from_slice(cue.as_bytes());
        }
        ShowControlCommand::LoadCue(cue) => {
            msg.push(0x11);
            msg.extend_from_slice(cue.as_bytes());
        }
    }
    msg.push(0xF7);
    msg
}
