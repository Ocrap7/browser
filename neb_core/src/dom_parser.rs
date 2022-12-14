use std::{
    collections::HashMap,
    io::{BufReader, Read},
};

use neb_errors::{DocumentError, DocumentErrorType};
use neb_graphics::drawing_context::DrawingContext;
use xml::{reader::XmlEvent, EventReader};

use crate::{
    is_node,
    node::{Node, NodeType},
    styling::{parse_styles, Selector},
    tree_display::TreeDisplay,
    Rf,
};

pub fn indent(size: usize) -> String {
    const INDENT: &'static str = "    ";
    (0..size)
        .map(|_| INDENT)
        .fold(String::with_capacity(size * INDENT.len()), |r, s| r + s)
}

pub struct Document {
    errors: Vec<DocumentError>,

    head: Rf<Node>,

    body: Rf<Node>,

    styles: Rf<HashMap<String, Rf<Selector>>>,
}

impl Document {
    pub fn get_errors(&self) -> &[DocumentError] {
        &self.errors
    }

    pub fn get_head(&self) -> Rf<Node> {
        self.head.clone()
    }

    pub fn get_body(&self) -> Rf<Node> {
        self.body.clone()
    }

    pub fn get_styles(&self) -> Rf<HashMap<String, Rf<Selector>>> {
        self.styles.clone()
    }
}

impl Document {
    pub fn draw(&self, dctx: &mut DrawingContext) {
        let body = self.body.borrow();
        body.draw(dctx, self);
    }
}

pub fn parse_from_stream<R>(stream: BufReader<R>) -> Document
where
    R: Read,
{
    let event_reader = EventReader::new(stream);

    let mut depth = 0;
    let mut nodes: HashMap<usize, Rf<Node>> = HashMap::new();
    let mut errors: Vec<DocumentError> = Vec::new();
    let mut styling = String::new();

    for e in event_reader {
        match e {
            Ok(XmlEvent::StartElement { name, .. }) => {
                if let Some(ty) = NodeType::try_node(name.local_name.as_str()) {
                    if depth == 0 {
                        nodes.insert(0, Rf::new(Node::new_root(ty)));
                    } else {
                        nodes.insert(
                            depth,
                            Rf::new(Node::new(ty, nodes.get(&(depth - 1)).unwrap().clone())),
                        );
                    }
                }
                println!("{}+{}", indent(depth), name);
                depth += 1;
            }
            Ok(XmlEvent::EndElement { name }) => {
                depth -= 1;
                if depth == 0 {
                    continue;
                }

                let Some(to_add) = nodes.remove(&depth) else {
                    continue;
                };

                println!("{}-{}", indent(depth), name);

                if let Some(node) = nodes.get_mut(&(depth - 1)) {
                    node.borrow_mut().add_child(to_add);
                }
            }
            Ok(XmlEvent::Characters(text)) => {
                let parent = nodes.get(&(depth - 1)).unwrap().clone();
                if let Some(node) = nodes.get_mut(&(depth - 1)) {
                    if is_node!(node.borrow(), NodeType::Style(_)) {
                        styling.push_str(text.trim());
                        styling.push('\n');
                        nodes.remove(&(depth - 1));
                        // *node = node
                        //     .clone()
                        //     .with_type(NodeType::Style(text.trim().to_string()));
                    } else {
                        let nd =
                            Rf::new(Node::new(NodeType::Text(text.trim().to_string()), parent));
                        node.borrow_mut().add_child_rf(nd);
                    }
                }
            }
            Err(e) => {
                println!("Error: {:#}", e);
                break;
            }
            _ => {}
        }
    }

    let (head, body) = if let Some(html) = nodes.get(&0) {
        let head = if let Some(head) = html.borrow().find_child_by_element_name("head") {
            head
        } else {
            errors.push(DocumentError::new(
                DocumentErrorType::ExpectedTag("head".into()),
                neb_errors::ErrorKind::Warning,
            ));
            Rf::new(Node::new_root(NodeType::Head))
        };

        let body = if let Some(body) = html.borrow().find_child_by_element_name("body") {
            body
        } else {
            errors.push(DocumentError::new(
                DocumentErrorType::ExpectedTag("body".into()),
                neb_errors::ErrorKind::Warning,
            ));
            Rf::new(Node::new_root(NodeType::Head))
        };

        (head, body)
    } else {
        errors.push(DocumentError::new(
            DocumentErrorType::ExpectedTag("html".into()),
            neb_errors::ErrorKind::Error,
        ));
        (
            Rf::new(Node::new_root(NodeType::Head)),
            Rf::new(Node::new_root(NodeType::Body)),
        )
    };

    // let p: String = head
    //     .borrow()
    //     .iter()
    //     .filter(|f| is_node!(f.borrow(), NodeType::Style(_)))
    //     .map(|f| {
    //         let style = f.borrow_mut();
    //         match style.get_type() {
    //             NodeType::Style(txt) => txt.clone(),
    //             _ => panic!(),
    //         }
    //     })
    //     .intersperse("\n".to_string())
    //     .collect();

    println!("{}", styling);
    println!("{}", head.borrow().format());
    println!("{}", body.borrow().format());
    styling.push('\0');

    let styles = parse_styles(styling.as_str()).unwrap();
    dbg!(&styles);



    Document {
        body,
        head,
        errors,
        styles: Rf::new(styles),
    }
}
