macro_rules! capture_single_var {
    ($name:ident) => {
        let $name = Clone::clone(&$name);
    };
    ($name:ident = $init:expr) => {
        let $name = Clone::clone(&$init);
    };
}

macro_rules! capture {
    (
        $(
            $name:ident $(= $init:expr)*
        ),+
        ;
        $closure:expr
    ) => {
        {
            $(
                capture_single_var!(
                    $name $(= $init)*
                );
            )+
            $closure
        }
    };
}