use std::borrow::Cow;

use swarm_common::constant::{RDF, RDFS, XSD};
use tortank::turtle::turtle_doc::{Node, Statement, TurtleDoc};

pub fn fix_triples(doc: TurtleDoc<'_>) -> anyhow::Result<TurtleDoc<'_>> {
    let mut new_stmts = vec![];
    for Statement {
        subject,
        predicate,
        object,
    } in doc
    {
        let (Some(subject), Some(predicate), Some(object)) =
            (fix_term(subject), fix_term(predicate), fix_term(object))
        else {
            continue;
        };
        new_stmts.push(Statement {
            subject,
            predicate,
            object,
        });
    }
    let doc: TurtleDoc = new_stmts.try_into().map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(doc)
}

// I was really lazy, so this is the bare minimum I think
fn fix_term(mut term: Node<'_>) -> Option<Node<'_>> {
    match term {
        Node::Iri(_) => Some(term),
        Node::Literal(ref mut literal) => match literal {
            tortank::turtle::turtle_doc::Literal::Quoted {
                datatype,
                value,
                lang,
            } => {
                lang.take(); // we don't want langStrings?
                if value.trim() != value {
                    *value = Cow::Owned(value.trim().to_owned());
                }
                match datatype {
                    Some(iri) => {
                        if iri.as_ref() == &Node::Iri(Cow::Owned(XSD("boolean"))) {
                            value
                                .to_lowercase()
                                .parse::<bool>()
                                .ok()
                                .map(tortank::turtle::turtle_doc::Literal::Boolean)
                                .map(Node::Literal)
                        } else if iri.as_ref() == &Node::Iri(Cow::Owned(RDFS("Literal")))
                            || iri.as_ref() == &Node::Iri(Cow::Owned(RDF("langString")))
                            || iri.as_ref() == &Node::Iri(Cow::Owned(RDF("XMLLiteral")))
                        {
                            *datatype = Some(Box::new(Node::Iri(Cow::Owned(XSD("string")))));
                            return Some(term);
                        } else {
                            Some(term)
                        }
                    }
                    None => Some(term),
                }
            }
            _ => Some(term),
        },
        Node::Ref(node) => {
            let node = &*node;
            fix_term(node.clone())
        }
        Node::LabeledBlankNode(_) | Node::List(_) => None,
    }
}
