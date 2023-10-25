use seed::{prelude::*, *};

pub fn view<Ms: Clone + 'static>(log: String) -> Node<Ms> {
    code![
        log.split("\n")
            .map(|line| div![line.replace(" ", " ")])
            .collect::<Vec<_>>(),
        style![St::Background => "#EEFFFFFF"]
    ]
}
