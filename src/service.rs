use crate::handler::{Factory, Handler};

#[derive(Debug, Clone, Copy)]
pub enum HttpStatus {
    Success,
    Failed,
}

pub struct BoxedService {
    service: Box<dyn Fn(Payload) -> HttpStatus>,
}

impl BoxedService {
    pub fn from_handler<F, Args, Res>(handler: Handler<F, Args, Res>) -> Self 
    where
        Args: FromPayload + 'static,
        Res: Into<HttpStatus> + 'static,
        F: Factory<Args, Res> + 'static,
    {
        let service = Box::new(move |mut payload| {
            match Args::from(&mut payload) {
                Ok(args) => handler.call(args).into(),
                Err(msg) => {
                    println!("{msg}");
                    HttpStatus::Failed
                }
            }
        });

        BoxedService { service }
    }

    pub fn handle(&self, payload: Payload) -> impl Into<HttpStatus> {
        (self.service)(payload)
    }
}

pub trait FromPayload: Sized {
    fn from(payload: &mut Payload) -> Result<Self, String>;
}

pub struct Payload {
    data: *const u8,
    len: usize,
}

impl Payload {
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        Payload {
            data: bytes.as_ptr(),
            len: bytes.len(),
        }
    }
}

trait Size: Sized {
    const SIZE: usize = std::mem::size_of::<Self>();
}
impl<T> Size for T {}

impl<T> FromPayload for T 
where T: BasicType
{
    fn from(payload: &mut Payload) -> Result<Self, String> {
        let payload_size = payload.len();

        if payload_size >= T::SIZE {
            unsafe {
                let t_ptr = payload.data as *const T;
                payload.data = payload.data.add(T::SIZE);
                Ok(t_ptr.read())
            }
        } else {
            Err("Failed to extract args from payload".into())
        }
    }
}

trait BasicType: Copy {}
macro_rules! mark_basic_type {
    ($($T: ident),+) => {$(
        impl BasicType for $T {}
    )+};
}

mark_basic_type!(
    f32, f64,
    i8, i16, i32, i64, isize,
    u8, u16, u32, u64, usize
);

impl FromPayload for () {
    fn from(_payload: &mut Payload) -> Result<Self, String> {
        Ok(())
    }
}

macro_rules! tuple_impl_from_payload {(  $( ( $($T: ident,)+ ) ),+ ) => 
    {$(
        impl<$($T),+> FromPayload for ($($T,)+) 
        where 
            $($T: FromPayload),+
        {
            #[allow(non_snake_case)]
            fn from(payload: &mut Payload) -> Result<Self, String> {
                $(let $T = $T::from(payload)?;)+
                Ok(($($T,)+))
            }
        }
    )+};
}

tuple_impl_from_payload!(
    (T0, ), 
    (T0, T1, ),
    (T0, T1, T2, ),
    (T0, T1, T2, T3, ),
    (T0, T1, T2, T3, T4, ),
    (T0, T1, T2, T3, T4, T5, ),
    (T0, T1, T2, T3, T4, T5, T6, ),
    (T0, T1, T2, T3, T4, T5, T6, T7, ),
    (T0, T1, T2, T3, T4, T5, T6, T7, T8, ),
    (T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, )
);

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn test_case_1() -> std::io::Result<()> {
        let mut buf = Vec::<u8>::new();
        buf.write(&10u8.to_le_bytes())?;
        buf.write(&10u16.to_le_bytes())?;
        buf.write(&10u32.to_le_bytes())?;
        buf.write(&10u64.to_le_bytes())?;
        buf.write(&10usize.to_le_bytes())?;
        buf.write(&10i8.to_le_bytes())?;
        buf.write(&10i16.to_le_bytes())?;
        buf.write(&10i32.to_le_bytes())?;
        buf.write(&10i64.to_le_bytes())?;
        buf.write(&10isize.to_le_bytes())?;

        let mut payload = Payload::from_bytes(&buf);

        let tuple = 
            <(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize) as FromPayload>::from(&mut payload).unwrap();
        assert_eq!(
            tuple,
            (10, 10, 10, 10, 10, 10, 10, 10, 10, 10)
        );
        Ok(())
    }

    #[test]
    fn test_case_2() -> std::io::Result<()> {
        let mut buf = Vec::<u8>::new();
        buf.write(&1.5f32.to_le_bytes())?;
        buf.write(&1.6f64.to_le_bytes())?;

        let mut payload = Payload::from_bytes(&buf);

        let tuple = 
            <(f32, f64) as FromPayload>::from(&mut payload).unwrap();
        assert_eq!(
            tuple,
            (1.5, 1.6)
        );
        Ok(())
    }
}