use super::*;
use glam::DVec2;

#[cfg(test)]
mod test;

pub trait Serde: Sized {
    fn serialize_to(&self, data: &mut Vec<u8>);
    fn deserialize_from(at: &mut usize, data: &[u8]) -> Self;

    fn serialize(&self) -> Vec<u8> {
        let mut vec = Vec::new();
        self.serialize_to(&mut vec);
        vec
    }

    fn deserialize(data: &[u8]) -> Self {
        Self::deserialize_from(&mut 0, data)
    }
}

impl Serde for f64 {
    fn serialize_to(&self, data: &mut Vec<u8>) {
        data.extend(self.to_le_bytes());
    }

    fn deserialize_from(at: &mut usize, data: &[u8]) -> Self {
        let my_area: [u8; 8] = data[*at..*at + 8]
            .try_into()
            .unwrap_or_else(|err| unreachable!("{err}"));
        *at += 8;
        f64::from_le_bytes(my_area)
    }
}

impl Serde for DVec2 {
    fn serialize_to(&self, data: &mut Vec<u8>) {
        self.x.serialize_to(data);
        self.y.serialize_to(data);
    }

    fn deserialize_from(at: &mut usize, data: &[u8]) -> Self {
        let x = f64::deserialize_from(at, data);
        let y = f64::deserialize_from(at, data);
        Self { x, y }
    }
}

impl<T: Serde> Serde for List<T> {
    fn serialize_to(&self, data: &mut Vec<u8>) {
        fn ser_len(mut to_encode: usize, data: &mut Vec<u8>) {
            let picked = (to_encode & 0b0011_1111) as u8;
            to_encode >>= 6;
            let continuing = to_encode > 0;
            data.push(picked | u8::from(continuing) << 6);
            while to_encode > 0 {
                let picked = (to_encode & 0b0111_1111) as u8;
                to_encode >>= 7;
                let continuing = to_encode > 0;
                data.push(picked | u8::from(continuing) << 7);
            }
        }

        match self {
            Self::Term(item) => {
                data.push(u8::MAX);
                item.serialize_to(data);
            }
            Self::Staggered(items) => {
                ser_len(items.len(), data);
                for item in items {
                    item.serialize_to(data);
                }
            }
            Self::Flat(items) => {
                ser_len(items.len(), data);
                for item in items {
                    data.push(u8::MAX);
                    item.serialize_to(data);
                }
            }
        }
    }

    fn deserialize_from(at: &mut usize, data: &[u8]) -> Self {
        let first_byte = data[*at];
        if first_byte >> 7 > 0 {
            *at += 1;
            Self::Term(T::deserialize_from(at, data))
        } else {
            let mut data_len = usize::from(first_byte & 0b0011_1111);
            *at += 1;

            let mut push_by = 6u32;
            let mut continued = first_byte >> 6 > 0;
            while continued {
                let byte = data[*at];
                continued = byte >> 7 > 0;
                data_len |= usize::from(byte & 0b0111_1111) << push_by;
                push_by += 7;
                *at += 1;
            }

            let mut items = Vec::with_capacity(data_len);
            for _ in 0..data_len {
                items.push(Self::deserialize_from(at, data));
            }

            List::list(items)
        }
    }
}

impl Serde for Value {
    fn serialize_to(&self, data: &mut Vec<u8>) {
        match self {
            Value::Number(x) => {
                data.push(0);
                x.serialize_to(data);
            }
            Value::Point(x) => {
                data.push(1);
                x.serialize_to(data);
            }
        }
    }

    fn deserialize_from(at: &mut usize, data: &[u8]) -> Self {
        let byte = data[*at];
        *at += 1;
        match byte {
            0 => Self::Number(<_>::deserialize_from(at, data)),
            _ => unreachable!(),
        }
    }
}
