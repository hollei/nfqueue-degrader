use std::array::TryFromSliceError;
use std::convert::TryInto;
use std::time::Duration;

#[derive(Clone, Default)]
pub struct Packet {
    pub number: u64,
    pub client_send_time: Duration,
    pub server_send_time: Duration,
    pub client_receive_time: Duration,
    pub payload: Vec<u8>,
}

impl Packet {
    pub fn from_bytes(mut data: &[u8]) -> Result<Packet, TryFromSliceError> {
        Ok(Packet {
            number: u64::read_be(&mut data)?,
            client_send_time: Duration::from_nanos(u128::read_be(&mut data)? as u64),
            server_send_time: Duration::from_nanos(u128::read_be(&mut data)? as u64),
            client_receive_time: Duration::from_nanos(u128::read_be(&mut data)? as u64),
            payload: data.to_owned(),
        })
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let mut data: Vec<u8> = Vec::with_capacity(8 + 16 + 16 + 16 + self.payload.len());
        data.extend_from_slice(&self.number.to_be_bytes());
        data.extend_from_slice(&self.client_send_time.as_nanos().to_be_bytes());
        data.extend_from_slice(&self.server_send_time.as_nanos().to_be_bytes());
        data.extend_from_slice(&self.client_receive_time.as_nanos().to_be_bytes());
        data.extend(&self.payload);
        data
    }

    pub fn rtt(&self) -> Duration {
        self.client_receive_time - self.client_send_time
    }
}

trait ReadBigEndian<T: TryInto<T>> {
    fn read_be(input: &mut &[u8]) -> Result<T, TryFromSliceError>;
}

impl ReadBigEndian<u64> for u64 {
    fn read_be(input: &mut &[u8]) -> Result<u64, TryFromSliceError> {
        let (head, rest) = input.split_at(std::mem::size_of::<u64>());
        *input = rest;
        Ok(u64::from_be_bytes(head.try_into()?))
    }
}

impl ReadBigEndian<u128> for u128 {
    fn read_be(input: &mut &[u8]) -> Result<u128, TryFromSliceError> {
        let (head, rest) = input.split_at(std::mem::size_of::<u128>());
        *input = rest;
        Ok(u128::from_be_bytes(head.try_into()?))
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn serialize_and_deserialize_packet() {
        let mut p = Packet::default();
        p.client_send_time = Duration::from_nanos(1234567000);
        p.client_receive_time = Duration::from_nanos(9871234000);
        p.server_send_time = Duration::from_nanos(876445000);
        let serialized = p.as_bytes();
        let p2 = Packet::from_bytes(serialized.as_slice()).unwrap();
        assert_eq!(p2.client_send_time, p.client_send_time);
        assert_eq!(p2.client_receive_time, p.client_receive_time);
        assert_eq!(p2.server_send_time, p.server_send_time);
    }
}
