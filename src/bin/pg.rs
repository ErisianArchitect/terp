
use ::core::{
    num::{
        NonZeroUsize,
    },
};

use terp::algo::{
    ParseNode,
    parse_interp,
    InterpError,
    interpolate,
    parse,
    Token,
    tokenize,
    FancyPerformer,
};

fn print_nodes(nodes: &[ParseNode<'_>], indent: usize, indent_width: usize, index: usize) {
    fn print_spaces(count: usize) {
        const SPACES: &'static str = "                ";
        let mut remain = count;
        while remain >= SPACES.len() {
            print!("{}", SPACES);
            remain -= SPACES.len();
        }
        if remain > 0 {
            print!("{}", &SPACES[..remain]);
        }
    }
    let mut i = 0usize;
    while i < nodes.len() {
        let node = nodes[i];
        match node {
            ParseNode::Interp(offset) => {
                let range = i+1..i+offset;
                print_nodes(&nodes[range], indent+indent_width, indent_width, index + i + 1);
                i += offset;
            },
            _ => {
                print_spaces(indent);
                println!("{:>02}: {node:?}", index + i);
                i += 1;
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct EnvPerformer {
    buffer: String,
}

impl<'a> FancyPerformer<'a> for EnvPerformer {
    type Error = ();
    fn visit_str(
        &mut self,
        raw: &str,
    ) -> Result<(), Self::Error> {
        self.buffer.push_str(raw);
        Ok(())
    }
    
    fn visit_dollar_id(&mut self, id: &'a str) -> Result<(),Self::Error> {
        if let Ok(value) = std::env::var(id) {
            return self.visit_str(value.as_str());
        }
        Ok(())
    }

    fn visit_braced_dollar(&mut self,tokens: &[Token<'a>],) -> Result<(),Self::Error> {
        let mut sub = Self::default();
        sub.visit_tokens(tokens)?;
        self.visit_dollar_id(sub.buffer.as_str())
    }
}

fn main() {
//     let text = r###"
// [test[test]]{test}This is a test{test with spaces} ${hmm} @hmm I wonder what this will do.$#
// "###.trim();
//     println!("{text}");
//     let tokens = match tokenize(text) {
//         Ok(tokens) => tokens,
//         Err(error) => {
//             eprintln!("{error}");
//             return;
//         }
//     };
//     println!("{tokens:#?}");
//     let mut buffer = String::with_capacity(1024);
//     buffer.visit_tokens(&tokens).expect("Failed to visit tokens.");
//     println!("{buffer}");
//     assert_eq!(buffer.as_str(), text);
    let mut env_perf = EnvPerformer::default();
    let format = "The value of foobar is \"${foobar}\"";
    let tokens = tokenize(format).expect("Failed to tokenize.");
    env_perf.visit_tokens(&tokens).expect("Failed to visit tokens");
    println!("{tokens:?}");
    println!("{}", env_perf.buffer);
    // let format = "Hello, %{item_%{w1}%}% and %{item_%{w2}%}%, this is a %{0}%";
    // let interp = interpolate(format, &mut |buf: &mut String, token: &str| {
    //     match token {
    //         "0" => buf.push_str("test"),
    //         "w1" => buf.push_str("world"),
    //         "w2" => buf.push_str("salad"),
    //         "item_world" => buf.push_str("World"),
    //         "item_salad" => buf.push_str("Salad"),
    //         _ => return Err(()),
    //     }
    //     Ok(())
    // }).expect("Failed to interpolate.");
    // println!("{interp}");
    // let format = "%{foo=David.}%Hello, %{foo}%\n%{foo=World}%Hello, %{foo}%.\n%{{}=%{{}=test}%This is a %{{}}%.}%[%{{}}%]";
    // let mut map = std::collections::HashMap::<Box<str>, Box<str>>::new();
    // let interp = interpolate(format, &mut move |buf, token| {
    //     if let Some(eq_ind) = token.find("=") {
    //         let name = &token[..eq_ind];
    //         let content = &token[eq_ind+1..];
    //         map.insert(name.into(), content.into());
    //     } else {
    //         if let Some(found) = map.get(token) {
    //             buf.push_str(found);
    //         } else {
    //             return Err(());
    //         }
    //     }
    //     Result::<(), ()>::Ok(())
    // }).expect("Failed to interpolate.");
    // println!("{interp}");
    // let format = "Hello, world!";
    // enum BracketStyle {
    //     Curly,
    //     Parens,
    //     Angle,
    //     Square,
    // }

    // for (i, token) in dest.into_iter().enumerate() {
    //     println!("{i:>02}: {token:?}");
    // }
    /*  ──[OUTPUT]──────────────────────────────────────────────────────
        Hello, David.
        Hello, World.
        [This is a test.]
        ────────────────────────────────────────────────────────────────  */
    // let mut nodes = Vec::new();
    // parse_interp(
    //     &mut nodes,
    //     "Hello, world! This is a %{nested_\\%{\\}%%{nest_value}%}%. This is the end. %{Left %{Inner Left %{middle}% Inner Right}% Right}%. Okay, for real this time."
    // ).expect("Failed to parse.");
    // print_nodes(&nodes, 0, 4, 0);
    // println!("----------------");
    // println!("{nodes:#?}");
}
