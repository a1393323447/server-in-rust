# Rust: 网络库的实现思路
这篇文章主要从 `Rust` 的类型系统的角度出发，提供一种网络库的实现思路。本文可以看作 `actix-web` 的阅读笔记。

## 对不同参数的函数和闭包的抽象
在进入正题之前，我们先看一段来自 `actix-web` 官方的示例代码:

```rust
use actix_web::{web, App, HttpResponse};

async fn index(data: web::Path<(String, String)>) -> &'static str {
    "Welcome!"
}

let app = App::new()
    .route("/test1", web::get().to(index))
    .route("/test2", web::get().to(|| HttpResponse::MethodNotAllowed()));
```

可以看到同样是 `get().to(..)`， 但却可以接受不同参数的函数和闭包。这也引出了我们对不同参数的函数、闭包进行抽象的需求。

我们定义一个 `trait` 名为 `Factory`:
```rust
pub trait Factory<Args, Res> {
    fn call(&self, args: Args) -> Res;
}
```

然后为实现了 `Fn trait` 的类型（因为该函数会被调用多次）， 实现 `Factory`:

```rust
impl<T, Res> Factory<(), Res> for T
where
    T: Fn() -> Res,
{
    fn call(&self, _args: ()) -> Res {
        (self)()
    }
}
```

但函数没有参数的时候，类型参数 `Args` 就为 `()` 。由于 `rust` 中缺少 `variadic generics`，所以当函数有一个或多个参数时，
我们需要将其打包成一个元组，再在调用的时候解包:
```rust
impl<T, Arg0, Res> Factory<(Arg0, ), Res> for T
where
    T: Fn(Arg0) -> Res,
{
    fn call(&self, args: (Arg0, )) -> Res {
        (self)(args.0)
    }
}

impl<T, Arg0, Arg1, Res> Factory<(Arg0, Arg1, ), Res> for T
where
    T: Fn(Arg0, Arg1) -> Res,
{
    fn call(&self, args: (Arg0, Arg1, )) -> Res {
        (self)(args.0, args.1)
    }
}
```
由于有大量重复代码，我们可以通过宏来实现这个过程:
```rust
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

// 由于难以在过程宏中得知这是第几次循环, 所以我们以元组的方式指示这个模板参数在元组中的哪一个位置
// 从而实现在调用对函数参数的解包 
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
```
至此, 我们完成了对不同参数(0 ~ 10)的函数和闭包的抽象。我们可以为其定义一个包装类 `Handler`:
```rust
pub struct Handler<F, A, R> {
    f: F,
    _t: PhantomData<(A, R)>,
}
```
其中 `A` 代表 `Args`， `R` 代表 `Res`，由于 `rust` 不允许游离的模板参数的存在，我们使用在 `Handler` 中定一个 `PhantomData<(A, R)>`。

接着为 `Handler` 实现一些方法:
```rust
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
```

## 反序列化
回到文章开头展示的 `actix-web` 的示例代码:
```rust
use actix_web::{web, App, HttpResponse};

async fn index(data: web::Path<(String, String)>) -> &'static str {
    "Welcome!"
}

let app = App::new()
    .route("/test1", web::get().to(index))
    .route("/test2", web::get().to(|| HttpResponse::MethodNotAllowed()));
```

我们看到函数可以接受不同类型的参数，而这些参数一般都是从发送到服务器的请求所携带的数据中得到的。所以我们还需要一个反序列化的 `trait`:

```rust
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
```
注意到 `FromPayload` 是受到 `Sized` 的限制的，因为我们需要将 `Result<Self, ...>` 作为返回值，
如果 `FromPayload` 没有受到 `Sized` 的限制，那么编译器就无法在编译时得知 `Self` 的大小，也就无法将 `Result<Self, ...>` 作为返回值。

而 `Payload` 结构体本质上是对字节流的抽象，因为这只是一个 `demo`，方便起见，我就使用裸指针。

接着，我们需要为一些基本的类型实现 `FromPayload`，如:
```rust
impl FromPayload for i32 {
    fn from(payload: &mut Payload) -> Result<Self, String> {
        let payload_size = payload.len();
        let t_size = std::mem::size_of<i32>();
        if payload_size >= t_size {
            unsafe {
                let t_ptr = payload.data as *const i32;
                payload.data = payload.data.add(t_size);
                Ok(t_ptr.read())
            }
        } else {
            Err("Failed to extract args from payload".into())
        }
    }
}
```
同样的，由于这个过程很繁琐我们可以通过宏实现:
```rust
// 为所有实现了 BasicType 的类型都实现 FromPayload
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

// BasicType 是一个 marker trait, 主要用于标记哪些类型是 BasicType
// 而之所以 `BasicType: Copy `, 是因为在 `FromPayload` 的实现中我们只是简单地进行字节的赋值
// 所以只有对于能进行 Copy 的类型来说, 上面的才是安全的
trait BasicType: Copy {}
macro_rules! mark_basic_type {
    ($($T: ident),+) => {$(
        impl BasicType for $T {}
    )+};
}

// 用宏减少重复代码
mark_basic_type!(
    f32, f64,
    i8, i16, i32, i64, isize,
    u8, u16, u32, u64, usize
);
```
而由于我们的 `handler` 所接受的参数都要被打包进一个元组里，所以我们也要为不同的元组实现 `FromPayload` , 
只要约束元组中的每一个类型实现 `FromPayload`，我们就为该元组实现 `FromPayload`:
```rust
impl FromPayload for () {
    fn from(_payload: &mut Payload) -> Result<Self, String> {
        Ok(())
    }
}

macro_rules! tuple_impl_from_payload {(  $( ( $($T: ident,)+ ) ),+ ) => 
    {$(
        impl<$($T),+> FromPayload for ($($T,)+) 
        where 
            // 约束元组中的每一个类型都实现 FromPayload
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
```
其中 `()` 要特别注意, 因为 `()` 只有一个实例，且为 `zero-sized type`，所以从字节流一定能得到它。

至此，我们已经可以从字节流中得到函数的参数了。

## 对不同类型的 `Handler` 进行抽象
让我们再次回到最开始的代码:
```rust
use actix_web::{web, App, HttpResponse};

async fn index(data: web::Path<(String, String)>) -> &'static str {
    "Welcome!"
}

let app = App::new()
    .route("/test1", web::get().to(index))
    .route("/test2", web::get().to(|| HttpResponse::MethodNotAllowed()));
```
我们看到在同一个 `App` 中是能够储存拥有不同参数和返回值的 `Handler` 的。而回到我们的 `Handler` 的定义:
```rust
pub struct Handler<F, A, R> {
    f: F,
    _t: PhantomData<(A, R)>,
}
```
可以看到 `Handler` 拥有三个类型参数，这也就意味着拥有不同参数、返回值的函数，甚至是同一参数、同一返回值的闭包都会导致
包含其的 `Handler` 是完全不同的类型。这也就意味着 `App` 中是不能直接储存这些 `Handler` 的。这也引出了一个新的需求：
对不同类型的 `Handler` 进行抽象。

我们来思考一下，`App` 是怎么样调用这些 `Handler` 的:
- 服务器接到请求
- 根据请求中包含的 `请求方式`、 `url` 等信息找到相应的处理函数
- 根据处理函数的签名从请求携带的数据中反序列化得到函数参数
- 调用函数

可以看到在调用函数之前我们需要通过反序列化得到函数参数，而我们已经通过 `FromPayload` 实现了对反序列化过程的抽象。
所以 `App` 主要做的步骤可以写为:
```rust
// 注意！这只是伪代码
impl App {
    fn handle_request(&self, req: Request) -> Response {
        let handler = match req.ty {
            RequestType::Get => self.get_handlers.get(&req.path),
            RequestType::Post => self.post_handlers.get(&req.path),
        };
        let args = <? as FromPayload>::from(req.payload());

        handler.call(args)
    }
}
```
在上面的代码中，我们发现在 `Handler` 被调用之前都需要使用 `FromPayload` 对 `req.payload()` 进行反序列化，
那么我们可以把这个过程和对 `Handler` 的调用封装成一个闭包:
```rust
// 注意！这只是伪代码
impl App {
    fn handle_request(&self, req: Request) -> Response {
        let handler = match req.ty {
            RequestType::Get => self.get_handlers.get(&req.path),
            RequestType::Post => self.post_handlers.get(&req.path),
        };
        let process = |payload: Payload| -> Response {
            let args = <? as FromPayload>::from(&mut payload);
            handler.call(args)
        };
        process(req.payload())
    }
}
```
那么更进一步，在一个函数被加入 `App` 的时候，我们就可以将其封装成一个闭包, 这样将可以将不类型的 `Handler` 统一起来了:
```rust
// 注意！这只是伪代码
impl App {
    fn get(&mut self, path: Path, h: Handler<F, A, R>) -> &mut self {
        let process = move |payload: Payload| -> Response {
            let args = <? as FromPayload>::from(&mut payload);
            h.call(args)
        };
        self.get_handlers.insert(path, process);

        self
    }
    fn handle_request(&self, req: Request) -> Response {
        let handler = match req.ty {
            RequestType::Get => self.get_handlers.get(&req.path),
            RequestType::Post => self.post_handlers.get(&req.path),
        };
        let process = |payload: Payload| {
            let args = <? as FromPayload>::from(&mut payload);
            handler.call(args)
        };
        process(request.payload())
    }
}
```

剩下还没有解决的问题就是我们如何得知 `Handler` 的函数参数类型了, 我们可以通过一个包装类实现:
```rust
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
```

## 最终实现
有了上面的铺垫，我们就可以实现最终的 `Server` 类了:

```rust
// file: src/request.rs
use crate::server::Path;

#[derive(Clone, Copy)]
pub enum RequestType {
    Get,
    Post,
}

pub struct Request {
    pub(crate) ty: RequestType,
    pub(crate) path: Path,
    pub(crate) payload: Vec<u8>,
}

impl Request {
    pub fn get(path: impl Into<Path>, payload: Vec<u8>) -> Request {
        Request { ty: RequestType::Get, path: path.into(), payload }
    }

    pub fn post(path: impl Into<Path>, payload: Vec<u8>) -> Request {
        Request { ty: RequestType::Post, path: path.into(), payload }
    }
}
```

```rust
// file: src/server.rs
use std::collections::HashMap;
use std::fmt::Display;

use crate::request::{Request, RequestType};
use crate::service::{BoxedService, HttpStatus, FromPayload, Payload};
use crate::handler::{Factory, Handler};

#[derive(Default)]
pub struct Server {
    get: HashMap<Path, BoxedService>,
    post: HashMap<Path, BoxedService>,
}

impl Server {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get<P, F, A, R>(&mut self, path: P, f: F) -> &mut Self 
    where
        P: Into<Path>,
        A: FromPayload + 'static,
        R: Into<HttpStatus> + 'static, 
        F: Factory<A, R> + 'static,
    {
        let handler = Handler::new(f);
        self.get.insert(path.into(), BoxedService::from_handler(handler));

        self
    }

    pub fn post<P, F, A, R>(&mut self, path: P, f: F) -> &mut Self 
    where
        P: Into<Path>,
        A: FromPayload + 'static,
        R: Into<HttpStatus> + 'static, 
        F: Factory<A, R> + 'static,
    {
        let handler = Handler::new(f);
        self.post.insert(path.into(), BoxedService::from_handler(handler));

        self
    }

    pub fn handle_request(&self, request: Request) -> Result<HttpStatus, String> {
        let service = match request.ty {
            RequestType::Get => match self.get.get(&request.path) {
                Some(s) => s,
                None => return Err(format!("missing get handler for path {}", request.path)), 
            },
            RequestType::Post => match self.post.get(&request.path) {
                Some(s) => s,
                None => return Err(format!("missing post handler for path {}", request.path)), 
            },
        };

        let payload = Payload::from_bytes(&request.payload);
        Ok(service.handle(payload).into())
    }
}


#[derive(PartialEq, Eq, Hash, Debug)]
pub struct Path {
    p: String,
}

impl Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.p)
    }
}

impl<T> From<T> for Path 
    where T: Into<String>
{
    fn from(value: T) -> Self {
        Path { p: value.into() }
    }
}
```
至此，我们的 `Server` 也完成了。

## 测试
剩下的最后一步，就是测试了:
```rust
#[cfg(test)]
mod tests {
    use std::io::Write;

    use crate::{server::*, service::HttpStatus, request::Request};

    fn success() -> HttpStatus {
        HttpStatus::Success
    }

    fn query_book(book_no: usize) -> impl Into<HttpStatus> {
        println!("query book no.{book_no} .");

        HttpStatus::Success
    }

    fn post_bill(bill_no: usize, price: f32) -> impl Into<HttpStatus> {
        println!("bill_no: {bill_no} price: {price}");

        HttpStatus::Success
    }

    #[test]
    fn it_works() {
        let mut server = Server::new();
        
        server
            .get("/", success)
            .get("/book", query_book)
            .post("/bill", post_bill);

        let requests = {
            let req1 = Request::get("/", vec![]);
            let req2 = Request::get("/book", 10usize.to_le_bytes().into());
            let req3 = {
                let mut buf = vec![];
                buf.write(&20usize.to_le_bytes()).unwrap();
                buf.write(&1.2f32.to_le_bytes()).unwrap();
                Request::post("/bill", buf)
            };
            let err_request1 = Request::get("/404", vec![]);
            let err_request2 = Request::post("/404", vec![]);

            [req1, req2, req3, err_request1, err_request2]
        };

        for request in requests {
            let res = server.handle_request(request);
            println!("{res:?}");
        }
    }
}
```
输出:
```
Ok(Success)
query book no.10 .
Ok(Success)
bill_no: 20 price: 1.2
Ok(Success)
Err("missing get handler for path /404\n")
Err("missing post handler for path /404\n")
```

## 总结
我个人认为这种模式不只是适用于网络库的构建，像 `bevy` 的 `ecs` 系统也可以用类似的方法实现。