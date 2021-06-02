use std::marker::PhantomData;

pub struct WithCtx<Ctx, F> {
    f: F,
    _phantom: PhantomData<Ctx>,
}
pub fn with_ctx<Ctx, F, T, E>(f: F) -> WithCtx<Ctx, F>
where
    F: Fn(&Ctx) -> Result<T, E>,
{
    WithCtx {
        f,
        _phantom: PhantomData,
    }
}
impl<Ctx, T, E, F> Transaction for WithCtx<Ctx, F>
where
    F: Fn(&Ctx) -> Result<T, E>,
{
    type Ctx = Ctx;
    type Item = T;
    type Err = E;

    fn run(&self, ctx: &Self::Ctx) -> Result<Self::Item, Self::Err> {
        (self.f)(ctx)
    }
}

pub trait Transaction {
    type Ctx;
    type Item;
    type Err;

    fn run(&self, ctx: &Self::Ctx) -> Result<Self::Item, Self::Err>;

    fn boxed<'a>(
        self,
    ) -> Box<dyn Transaction<Ctx = Self::Ctx, Item = Self::Item, Err = Self::Err> + 'a>
    where
        Self: Sized + 'a,
    {
        Box::new(self)
    }
}
