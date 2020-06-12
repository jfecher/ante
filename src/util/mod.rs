
pub fn fmap<T, U, F>(array: &[T], mut f: F) -> Vec<U>
    where F: FnMut(&T) -> U
{
    array.iter().map(|x| f(x)).collect()
}

pub fn fmap_mut<T, U, F>(array: &mut [T], mut f: F) -> Vec<U>
    where F: FnMut(&mut T) -> U
{
    array.iter_mut().map(|x| f(x)).collect()
}

pub fn fmap2<Elem1, Elem2, Ret, F>(array1: &[Elem1], array2: &[Elem2], mut f: F) -> Vec<Ret>
    where F: FnMut(&Elem1, &Elem2) -> Ret
{
    let second_iter = array2.iter();
    array1.iter().zip(second_iter).map(|(x, y)| f(x, y)).collect()
}

pub fn contains<T: PartialEq>(array: &[T], element: &T) -> bool {
    array.iter().find(|&x| x == element).is_some()
}
