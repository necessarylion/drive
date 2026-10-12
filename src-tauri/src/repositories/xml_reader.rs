use roxmltree::Node;

/// Small helpers to read the XML answers of S3 and Azure.
pub struct XmlReader;

impl XmlReader {
    /// The child elements with this name. The XML namespace does not matter.
    pub fn children<'a>(node: Node<'a, 'a>, name: &str) -> Vec<Node<'a, 'a>> {
        let mut found = Vec::new();
        for child in node.children() {
            if child.is_element() && child.tag_name().name() == name {
                found.push(child);
            }
        }
        found
    }

    pub fn child<'a>(node: Node<'a, 'a>, name: &str) -> Option<Node<'a, 'a>> {
        Self::children(node, name).into_iter().next()
    }

    /// The text of the first child with this name, or "".
    pub fn text<'a>(node: Node<'a, 'a>, name: &str) -> &'a str {
        Self::child(node, name).and_then(|child| child.text()).unwrap_or("")
    }
}
