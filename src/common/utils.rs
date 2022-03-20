use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

type RcCellType<T> = Rc<RefCell<T>>;

pub struct RcCell<T> {
    cell: RcCellType<T>,
}

impl<T> RcCell<T> {
    pub fn new(data: T) -> Self {
        Self {
            cell: Rc::new(RefCell::new(data)),
        }
    }
    pub fn as_mut(&self) -> RefMut<T> {
        self.cell.as_ref().borrow_mut()
    }

    pub fn as_ref(&self) -> Ref<T> {
        self.cell.as_ref().borrow()
    }
}

impl<T> Clone for RcCell<T> {
    fn clone(&self) -> Self {
        Self {
            cell: self.cell.clone(),
        }
    }
}
