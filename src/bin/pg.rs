
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
    TokenVisitor,
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
pub struct EnvVisitor {
    buffer: String,
}

impl<'a> TokenVisitor<'a> for EnvVisitor {
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

    fn visit_esc(
        &mut self,
        esc: &'a str,
    ) -> terp::algo::Result<(), Self::Error> {
        let Ok(esc) = terp::util::escape_sequence(esc) else {
            return Err(());
        };
        self.visit_str(esc.encode_utf8(&mut [0u8; 4]))
    }
}

fn main() {
    let mut env_perf = EnvVisitor::default();
    let format = r###"
{@
    This is a block.
    \x7a
    \x7A
}
\@foo #[left #[left inner #[%(middle)] right inner] right]
The \x76alue of foo${foo_end} is "${foo${foo_end}}"\nSession Name: $SESSION_NAME
"###.trim();
    let tokens = tokenize(format).expect("Failed to tokenize.");
    let mut rebuilt = String::with_capacity(format.len());
    rebuilt.visit_tokens(&tokens).expect("Failed to visit tokens. rebuilt");
    env_perf.visit_tokens(&tokens).expect("Failed to visit tokens. env_perf");
    assert_eq!(rebuilt, format);
    println!("-[tokens]-------------------------------------------------------");
    println!("{tokens:?}");
    println!("-[rebuilt]------------------------------------------------------");
    println!("{rebuilt}");
    println!("-[interpolated]-------------------------------------------------");
    println!("{}", env_perf.buffer);
    println!("----------------------------------------------------------------");
}
