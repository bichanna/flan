/// Accepts a vector of T and creates a String
#[macro_export]
macro_rules! bulk_print {
    ($vec:expr, $s:expr) => {
        $vec.iter()
            .map(|x| x.print())
            .collect::<Vec<String>>()
            .join($s)
    };
}

#[macro_export]
macro_rules! parse {
    ($source:expr, $expected:expr) => {
        let source = String::from($source);
        // for tokenizing
        let (ts, tr) = crossbeam_channel::unbounded();
        // for parsing
        let (ps, pr) = crossbeam_channel::unbounded();

        std::thread::scope(|s| {
            s.spawn(|| {
                Lexer::new(&source, "input", &ts);
            });

            s.spawn(|| {
                Parser::new(&source, "input", &tr, &ps);
            });
        });

        let mut exprs: Vec<Expr> = vec![];
        loop {
            let expr = pr.recv().unwrap();
            match expr {
                Expr::End => break,
                _ => {}
            }
            exprs.push(expr);
        }

        let result = Expr::pretty_print(&exprs);
        assert_eq!(result, $expected);
    };
}
