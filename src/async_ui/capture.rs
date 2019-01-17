macro_rules! capture_single_var {
    ($name:ident) => {
        let $name = $name.clone();
    };
    ($name:ident = $init:expr) => {
        let $name = $init.clone();
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
            )+;
            $closure
        }
    };
}