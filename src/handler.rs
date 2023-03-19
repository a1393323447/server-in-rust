use std::marker::PhantomData;

pub trait Factory<Args, Res> {
    fn call(&self, args: Args) -> Res;
}

pub struct Handler<F, A, R> {
    f: F,
    _t: PhantomData<(A, R)>,
}

impl<F, A, R> Clone for Handler<F, A, R> 
    where F: Clone,
{
    fn clone(&self) -> Self {
        Self { f: self.f.clone(), _t: self._t.clone() }
    }
}

impl<F, A, R> Handler<F, A, R>
where
    F: Factory<A, R>,
{
    pub fn new(f: F) -> Self {
        Handler {
            f,
            _t: PhantomData::default(),
        }
    }

    pub fn call(&self, args: A) -> R {
        self.f.call(args)
    }
}

impl<T, Res> Factory<(), Res> for T
where
    T: Fn() -> Res,
{
    fn call(&self, _args: ()) -> Res {
        (self)()
    }
}

macro_rules! factory_tuple {( $(($arg: ident, $n: tt)),+ ) => {
        impl<T, $($arg,)+ Res> Factory<($($arg,)+), Res> for T
            where T: Fn($($arg,)+) -> Res
        {
            fn call(&self, args: ($($arg,)+)) -> Res {
                (self)($(args.$n,)+)
            }
        }
    };
}

factory_tuple!((Arg0, 0));
factory_tuple!((Arg0, 0), (Arg1, 1));
factory_tuple!((Arg0, 0), (Arg1, 1), (Arg2, 2));
factory_tuple!((Arg0, 0), (Arg1, 1), (Arg2, 2), (Arg3, 3));
factory_tuple!((Arg0, 0), (Arg1, 1), (Arg2, 2), (Arg3, 3), (Arg4, 4));
factory_tuple!((Arg0, 0), (Arg1, 1), (Arg2, 2), (Arg3, 3), (Arg4, 4), (Arg5, 5));
factory_tuple!((Arg0, 0), (Arg1, 1), (Arg2, 2), (Arg3, 3), (Arg4, 4), (Arg5, 5), (Arg6, 6));
factory_tuple!((Arg0, 0), (Arg1, 1), (Arg2, 2), (Arg3, 3), (Arg4, 4), (Arg5, 5), (Arg6, 6), (Arg7, 7));
factory_tuple!((Arg0, 0), (Arg1, 1), (Arg2, 2), (Arg3, 3), (Arg4, 4), (Arg5, 5), (Arg6, 6), (Arg7, 7), (Arg8, 8));
factory_tuple!((Arg0, 0), (Arg1, 1), (Arg2, 2), (Arg3, 3), (Arg4, 4), (Arg5, 5), (Arg6, 6), (Arg7, 7), (Arg8, 8), (Arg9, 9));
