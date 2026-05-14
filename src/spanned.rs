use core::cell::Cell;
use std::cell::RefCell;

thread_local! {
    static LAST_PARSE_OFFSET: Cell<usize> = Cell::new(0);

    static STACK: RefCell<Vec<usize>> = RefCell::new(Vec::new());
    static STACK_CURSOR: Cell<usize> = Cell::new(0);
}

/// A span of text in the input, represented by the byte indices of its start and end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub from: usize,
    pub to: usize,
    _private: (),
}

/// A value of type `T` that was parsed from a span of text in the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)] //
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

pub struct SpannedOffsetGuard;

pub(crate) fn start_span(offset: usize) -> SpannedOffsetGuard {
    STACK.with_borrow_mut(|stack| {
        let cursor = STACK_CURSOR.get();
        STACK_CURSOR.set(cursor + 1);

        stack.truncate(cursor);
        stack.push(offset);
    });
    SpannedOffsetGuard
}

impl SpannedOffsetGuard {
    pub fn end(self, offset: usize) {
        LAST_PARSE_OFFSET.set(offset);
        STACK_CURSOR.set(STACK_CURSOR.get() - 1);

        std::mem::forget(self)
    }
}

impl Drop for SpannedOffsetGuard {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        panic!("SpannedOffsetGuard was dropped without calling end()")
    }
}

impl Span {
    /// Creates a new span from the given start and end indices.
    pub(crate) const fn new(from: usize, to: usize) -> Self {
        Self {
            from,
            to,
            _private: (),
        }
    }
}

impl<T: serde::Serialize> serde::Serialize for Spanned<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(serializer)
    }
}

impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for Spanned<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let start_cursor = STACK_CURSOR.get();
        let value = T::deserialize(deserializer)?;

        let span = STACK.with_borrow(|stack| {
            let from = stack[start_cursor];
            let to = LAST_PARSE_OFFSET.get();

            Span::new(from, to)
        });

        Ok(Self { value, span })
    }
}
