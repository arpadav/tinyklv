use num_traits::Unsigned;

struct Klv<K, L, V>
where 
    L: Unsigned,
{
    key: K,
    len: L,
    val: V,
}