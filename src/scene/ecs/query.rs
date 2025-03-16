use crate::scene::ecs::{Comp, World};
use std::any::Any;
use std::borrow::Borrow;
use std::marker::PhantomData;

trait QueryComp<'a> {
    type Item: Comp;
    fn parse(item: &'a mut Option<Box<dyn Any>>) -> Option<Self>
    where
        Self: Sized;
}

impl<'a, C: Comp> QueryComp<'a> for &'a C {
    type Item = C;

    fn parse(item: &'a mut Option<Box<dyn Any>>) -> Option<Self> {
        match item {
            None => None,
            Some(v) => v.downcast_ref::<C>(),
        }
    }
}

impl<'a, C: Comp> QueryComp<'a> for &'a mut C {
    type Item = C;

    fn parse(item: &'a mut Option<Box<dyn Any>>) -> Option<Self> {
        match item {
            None => None,
            Some(v) => v.downcast_mut::<C>(),
        }
    }
}
impl<'a, C: Comp> QueryComp<'a> for Option<&'a C> {
    type Item = C;
    fn parse(item: &'a mut Option<Box<dyn Any>>) -> Option<Self> {
        match item {
            None => Some(None),
            Some(v) => Some(v.downcast_ref::<C>()),
        }
    }
}
impl<'a, C: Comp> QueryComp<'a> for Option<&'a mut C> {
    type Item = C;
    fn parse(item: &'a mut Option<Box<dyn Any>>) -> Option<Self> {
        match item {
            None => Some(None),
            Some(v) => Some(v.downcast_mut::<C>()),
        }
    }
}

#[derive(Debug, Clone)]
struct QueryItemGetInvalid;
type QueryItemResult<T> = Result<T, QueryItemGetInvalid>;
type QueryData = Vec<*mut Vec<Option<Box<dyn Any>>>>;

pub trait QueryItem {
    fn fetch(world: &mut World) -> Option<QueryData>;
    fn try_get(data: &mut QueryData, index: usize) -> QueryItemResult<Self>
    where
        Self: Sized;
}

impl<'a, T1: QueryComp<'a>> QueryItem for T1 {
    fn fetch(world: &mut World) -> Option<QueryData> {
        let item1 = &mut *world.get_comps_mut::<T1::Item>()? as *mut Vec<_>;
        Some(vec![item1])
    }

    fn try_get(data: &mut QueryData, index: usize) -> QueryItemResult<Self> {
        unsafe {
            let item1 =
                T1::parse((*data[0]).get_unchecked_mut(index)).ok_or(QueryItemGetInvalid)?;
            Ok(item1)
        }
    }
}

impl<'a, T1: QueryComp<'a>, T2: QueryComp<'a>> QueryItem for (T1, T2) {
    fn fetch(world: &mut World) -> Option<QueryData> {
        let item1 = &mut *world.get_comps_mut::<T1::Item>()? as *mut Vec<_>;
        let item2 = &mut *world.get_comps_mut::<T2::Item>()? as *mut Vec<_>;

        Some(vec![item1, item2])
    }

    fn try_get(data: &mut QueryData, index: usize) -> QueryItemResult<Self> {
        unsafe {
            let item1 =
                T1::parse((*data[0]).get_unchecked_mut(index)).ok_or(QueryItemGetInvalid)?;
            let item2 =
                T2::parse((*data[1]).get_unchecked_mut(index)).ok_or(QueryItemGetInvalid)?;

            Ok((item1, item2))
        }
    }
}

impl<'a, T1: QueryComp<'a>, T2: QueryComp<'a>, T3: QueryComp<'a>> QueryItem for (T1, T2, T3) {
    fn fetch(world: &mut World) -> Option<QueryData> {
        let item1 = &mut *world.get_comps_mut::<T1::Item>()? as *mut Vec<_>;
        let item2 = &mut *world.get_comps_mut::<T2::Item>()? as *mut Vec<_>;
        let item3 = &mut *world.get_comps_mut::<T3::Item>()? as *mut Vec<_>;

        Some(vec![item1, item2, item3])
    }

    fn try_get(data: &mut QueryData, index: usize) -> QueryItemResult<Self> {
        unsafe {
            let item1 =
                T1::parse((*data[0]).get_unchecked_mut(index)).ok_or(QueryItemGetInvalid)?;
            let item2 =
                T2::parse((*data[1]).get_unchecked_mut(index)).ok_or(QueryItemGetInvalid)?;
            let item3 =
                T3::parse((*data[2]).get_unchecked_mut(index)).ok_or(QueryItemGetInvalid)?;

            Ok((item1, item2, item3))
        }
    }
}

pub struct Query<T, S = ()> {
    data: Option<QueryData>,
    count: usize,
    curr: usize,
    phantom: PhantomData<(T, S)>,
}

impl<T, S> Query<T, S>
where
    T: QueryItem,
{
    pub fn new(world: &mut World) -> Query<T, S> {
        Self {
            data: T::fetch(world),
            count: world.entity_count(),
            curr: 0,
            phantom: PhantomData,
        }
    }
}

impl<T, S> Iterator for Query<T, S>
where
    T: QueryItem,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.is_none() {
            return None;
        }

        while self.curr < self.count {
            let result = T::try_get(&mut self.data.as_mut().unwrap(), self.curr);
            self.curr = self.curr + 1;

            match result {
                Ok(v) => return Some(v),
                Err(_) => {}
            }
        }

        None
    }
}
