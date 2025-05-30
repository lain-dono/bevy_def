#[cfg(debug_assertions)]
#[inline(always)]
#[track_caller]
pub(crate) unsafe fn debug_checked_unwrap_result<T, E>(result: Result<T, E>) -> T {
    if let Ok(inner) = result {
        inner
    } else {
        unreachable!()
    }
}

#[cfg(not(debug_assertions))]
#[inline(always)]
#[track_caller]
pub(crate) unsafe fn debug_checked_unwrap_result<T, E>(result: Result<T, E>) -> T {
    if let Ok(inner) = result {
        inner
    } else {
        core::hint::unreachable_unchecked()
    }
}

#[cfg(debug_assertions)]
#[inline(always)]
#[track_caller]
pub(crate) unsafe fn debug_checked_unwrap_option<T>(option: Option<T>) -> T {
    if let Some(inner) = option {
        inner
    } else {
        unreachable!()
    }
}

#[cfg(not(debug_assertions))]
#[inline(always)]
#[track_caller]
pub(crate) unsafe fn debug_checked_unwrap_option<T>(option: Option<T>) -> T {
    if let Some(inner) = option {
        inner
    } else {
        core::hint::unreachable_unchecked()
    }
}
