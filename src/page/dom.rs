use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

use html5ever::tendril::StrTendril;
use html5ever::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::{Attribute, ExpandedName, QualName};

pub(super) type Handle = Rc<Node>;

#[derive(Debug)]
pub(super) struct Node {
    pub(super) parent: RefCell<Option<Weak<Node>>>,
    pub(super) children: RefCell<Vec<Handle>>,
    pub(super) data: NodeData,
}

#[derive(Debug)]
pub(super) enum NodeData {
    Document,
    Element {
        name: QualName,
        attributes: RefCell<Vec<Attribute>>,
        template_contents: Handle,
    },
    Text(RefCell<StrTendril>),
    Other,
}

impl Node {
    fn new(data: NodeData) -> Handle {
        Rc::new(Self {
            parent: RefCell::new(None),
            children: RefCell::new(Vec::new()),
            data,
        })
    }
}

pub(super) struct Dom {
    pub(super) document: Handle,
}

impl Default for Dom {
    fn default() -> Self {
        Self {
            document: Node::new(NodeData::Document),
        }
    }
}

impl TreeSink for Dom {
    type Handle = Handle;
    type Output = Self;
    type ElemName<'a> = ExpandedName<'a>;

    fn finish(self) -> Self::Output {
        self
    }

    fn parse_error(&self, _message: Cow<'static, str>) {}

    fn get_document(&self) -> Self::Handle {
        Rc::clone(&self.document)
    }

    fn get_template_contents(&self, target: &Self::Handle) -> Self::Handle {
        match &target.data {
            NodeData::Element {
                template_contents, ..
            } => Rc::clone(template_contents),
            _ => panic!("template contents require an element"),
        }
    }

    fn set_quirks_mode(&self, _mode: QuirksMode) {}

    fn same_node(&self, left: &Self::Handle, right: &Self::Handle) -> bool {
        Rc::ptr_eq(left, right)
    }

    fn elem_name<'a>(&'a self, target: &'a Self::Handle) -> Self::ElemName<'a> {
        match &target.data {
            NodeData::Element { name, .. } => name.expanded(),
            _ => panic!("element name requires an element"),
        }
    }

    fn create_element(
        &self,
        name: QualName,
        attributes: Vec<Attribute>,
        _flags: ElementFlags,
    ) -> Self::Handle {
        Node::new(NodeData::Element {
            name,
            attributes: RefCell::new(attributes),
            template_contents: Node::new(NodeData::Document),
        })
    }

    fn create_comment(&self, _text: StrTendril) -> Self::Handle {
        Node::new(NodeData::Other)
    }

    fn create_pi(&self, _target: StrTendril, _data: StrTendril) -> Self::Handle {
        Node::new(NodeData::Other)
    }

    fn append(&self, parent: &Self::Handle, child: NodeOrText<Self::Handle>) {
        append_child(parent, child);
    }

    fn append_before_sibling(&self, sibling: &Self::Handle, child: NodeOrText<Self::Handle>) {
        let parent = parent(sibling).expect("sibling insertion requires a parent");
        let index = parent
            .children
            .borrow()
            .iter()
            .position(|candidate| Rc::ptr_eq(candidate, sibling))
            .expect("sibling must belong to its parent");
        if let NodeOrText::AppendText(text) = &child
            && index > 0
            && append_to_text(&parent.children.borrow()[index - 1], text)
        {
            return;
        }
        let child = into_node(child);
        detach(&child);
        *child.parent.borrow_mut() = Some(Rc::downgrade(&parent));
        parent.children.borrow_mut().insert(index, child);
    }

    fn append_based_on_parent_node(
        &self,
        element: &Self::Handle,
        previous_element: &Self::Handle,
        child: NodeOrText<Self::Handle>,
    ) {
        if parent(element).is_some() {
            self.append_before_sibling(element, child);
        } else {
            self.append(previous_element, child);
        }
    }

    fn append_doctype_to_document(
        &self,
        _name: StrTendril,
        _public_id: StrTendril,
        _system_id: StrTendril,
    ) {
    }

    fn add_attrs_if_missing(&self, target: &Self::Handle, attributes: Vec<Attribute>) {
        let NodeData::Element {
            attributes: current,
            ..
        } = &target.data
        else {
            panic!("attributes require an element");
        };
        let mut current = current.borrow_mut();
        for attribute in attributes {
            if !current.iter().any(|item| item.name == attribute.name) {
                current.push(attribute);
            }
        }
    }

    fn remove_from_parent(&self, target: &Self::Handle) {
        detach(target);
    }

    fn reparent_children(&self, node: &Self::Handle, new_parent: &Self::Handle) {
        let children = std::mem::take(&mut *node.children.borrow_mut());
        for child in children {
            *child.parent.borrow_mut() = Some(Rc::downgrade(new_parent));
            new_parent.children.borrow_mut().push(child);
        }
    }
}

fn append_child(parent: &Handle, child: NodeOrText<Handle>) {
    if let NodeOrText::AppendText(text) = &child
        && parent
            .children
            .borrow()
            .last()
            .is_some_and(|last| append_to_text(last, text))
    {
        return;
    }
    let child = into_node(child);
    detach(&child);
    *child.parent.borrow_mut() = Some(Rc::downgrade(parent));
    parent.children.borrow_mut().push(child);
}

fn into_node(child: NodeOrText<Handle>) -> Handle {
    match child {
        NodeOrText::AppendNode(node) => node,
        NodeOrText::AppendText(text) => Node::new(NodeData::Text(RefCell::new(text))),
    }
}

fn append_to_text(node: &Handle, text: &StrTendril) -> bool {
    let NodeData::Text(contents) = &node.data else {
        return false;
    };
    contents.borrow_mut().push_tendril(text);
    true
}

fn parent(node: &Handle) -> Option<Handle> {
    node.parent.borrow().as_ref().and_then(Weak::upgrade)
}

fn detach(node: &Handle) {
    let Some(parent) = parent(node) else {
        return;
    };
    if let Some(index) = parent
        .children
        .borrow()
        .iter()
        .position(|candidate| Rc::ptr_eq(candidate, node))
    {
        parent.children.borrow_mut().remove(index);
    }
    *node.parent.borrow_mut() = None;
}
