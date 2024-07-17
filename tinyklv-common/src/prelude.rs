macro_rules! xcodify {
    ($($name:ident),*) => {
        pub enum Codec {
            Decoder(KlvTypes),
            Encoder(KlvTypes),
        }
        pub enum KlvTypes {
            $(
                $name
            ),*
        }
        $(
            pub enum $name {
                Decoder,
                Encoder,
            }
            impl Into<Codec> for $name {
                fn into(self) -> Codec {
                    match self {
                        $name::Decoder => Codec::Decoder(KlvTypes::$name),
                        $name::Encoder => Codec::Encoder(KlvTypes::$name),
                    }
                }
            }
        )*
    };
}

xcodify! {
    Key,
    Len,
    Val,
    DefaultType
}