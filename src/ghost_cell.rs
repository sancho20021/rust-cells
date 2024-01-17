use ghost_cell::{GhostCell, GhostToken};
use std::{
    fmt::{Debug, Display, Pointer},
    sync::{Arc, Weak},
};

/*
This is not my implementation. Source: https://gitlab.mpi-sws.org/FP/ghostcell/-/blob/master/ghostcell/src/dlist_arc.rs
*/
/// A doubly-linked list node.
pub struct Node<'id, T> {
    data: T,
    prev: Option<WeakNodePtr<'id, T>>,
    next: Option<NodePtr<'id, T>>,
}
/// A `Weak` pointer to a node.
pub type WeakNodePtr<'id, T> = Weak<GhostCell<'id, Node<'id, T>>>;
/// A strong `Arc` pointer to a node.
pub type NodePtr<'id, T> = Arc<GhostCell<'id, Node<'id, T>>>;

impl<'id, T> Node<'id, T> {
    pub fn new(value: T) -> NodePtr<'id, T> {
        Arc::new(GhostCell::new(Self {
            data: value,
            prev: None,
            next: None,
        }))
    }

    pub fn prev_weak(&self) -> Option<&WeakNodePtr<'id, T>> {
        self.prev.as_ref()
    }

    pub fn prev(&self) -> Option<NodePtr<'id, T>> {
        self.prev_weak().and_then(|p| p.upgrade())
    }

    pub fn next(&self) -> Option<&NodePtr<'id, T>> {
        self.next.as_ref()
    }

    /// Unlink the nodes adjacent to `node`. The node will have `next` and `prev` be `None` after this.
    pub fn remove<'a>(node: &NodePtr<'id, T>, token: &'a mut GhostToken<'id>) {
        // `take` both pointers from `node`, setting its fields to `None`.
        let node = node.borrow_mut(token);
        let old_prev: Option<NodePtr<'id, T>> = node.prev.take().and_then(|p| p.upgrade());
        let old_next: Option<NodePtr<'id, T>> = node.next.take();
        // link `old_prev` and `old_next together
        if let Some(old_next) = &old_next {
            old_next.borrow_mut(token).prev = old_prev.as_ref().map(|p| Arc::downgrade(&p));
        }
        if let Some(old_prev) = &old_prev {
            old_prev.borrow_mut(token).next = old_next;
        }
    }

    /// Insert `node2` right after `node1` in the list.
    pub fn insert_next<'a>(
        node1: &NodePtr<'id, T>,
        node2: NodePtr<'id, T>,
        token: &'a mut GhostToken<'id>,
    ) {
        // Step 1: unlink the prev and next pointers of nodes that are
        // adjacent to node2.
        Self::remove(&node2, token);

        // Step 2: get out the old next pointer as node1_old_next.
        let node1_old_next: Option<NodePtr<'id, T>> = node1.borrow_mut(token).next.take();
        if let Some(node1_old_next) = &node1_old_next {
            node1_old_next.borrow_mut(token).prev = Some(Arc::downgrade(&node2));
        }

        // Step 3: link node2 to node1 and node1_old_next.
        let node2_inner: &mut Node<'id, T> = node2.borrow_mut(token);
        node2_inner.prev = Some(Arc::downgrade(node1));
        node2_inner.next = node1_old_next;

        // Step 4: Link node1.next to node2.
        node1.borrow_mut(token).next = Some(node2);
    }

    /// Construct an imutable iterator to traverse immutably.
    pub fn iter<'iter>(
        node: &'iter NodePtr<'id, T>,
        token: &'iter GhostToken<'id>,
    ) -> Iter<'id, 'iter, T> {
        Iter {
            cur: Some(node.as_ref()),
            token,
        }
    }

    /// Mutable iteration only works as "interior iteration", since we cannot hand out mutable references
    /// to multiple nodes at the same time.
    pub fn iter_mut(
        node: &NodePtr<'id, T>,
        token: &mut GhostToken<'id>,
        mut f: impl FnMut(&mut T),
    ) {
        let mut cur: Option<NodePtr<'id, T>> = Some(node.clone());
        while let Some(node) = cur {
            let node: &mut Node<'id, T> = node.borrow_mut(token); // mutably borrow `node` with `token`
            f(&mut node.data);
            cur = node.next.clone();
        }
    }

    /// Immutable interior traversal.
    pub fn iterate(node: &NodePtr<'id, T>, token: &GhostToken<'id>, f: impl Fn(&T)) {
        let mut cur: Option<&GhostCell<'id, Node<'id, T>>> = Some(node.as_ref());
        while let Some(node) = cur {
            let node: &Node<'id, T> = node.borrow(token); // immutably borrow `node` with `token`
            f(&node.data);
            cur = node.next.as_deref();
        }
    }

    pub fn view_as_vec<'a>(node: &'a NodePtr<'id, T>, token: &'a GhostToken<'id>) -> Vec<&'a T> {
        Node::iter(node, token).collect::<Vec<_>>()
    }
}

/// An immutable iterator.
pub struct Iter<'id, 'iter, T> {
    cur: Option<&'iter GhostCell<'id, Node<'id, T>>>,
    token: &'iter GhostToken<'id>,
}

impl<'id, 'iter, T> Iterator for Iter<'id, 'iter, T>
where
    T: 'iter,
{
    type Item = &'iter T;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.cur {
            let node: &Node<'id, T> = node.borrow(self.token); // immutably borrow `node` with `token`
            self.cur = node.next.as_deref();
            Some(&node.data)
        } else {
            None
        }
    }
}

fn init_list<'id>(
    token: &mut GhostToken<'id>,
    list_size: i32,
) -> (NodePtr<'id, i32>, NodePtr<'id, i32>) {
    let head: NodePtr<i32> = Node::new(0);
    let mut tail = head.clone();

    // To append to the list, we need a &mut GhostToken
    for i in 1..list_size {
        let node = Node::new(i);
        Node::insert_next(&tail, node.clone(), token);
        tail = node;
    }

    (head, tail)
}

fn print_list<'id, T: std::fmt::Debug>(list: &NodePtr<'id, T>, token: &GhostToken<'id>) {
    println!(
        "{}",
        Node::iter(list, token)
            .map(|n| format!("{:?}", n))
            .collect::<Vec<_>>()
            .join(", ")
    );
}

struct ListWrapper<'id, T> {
    head: NodePtr<'id, T>,
    token: GhostToken<'id>,
}

impl<'id, T> ListWrapper<'id, T> {
    pub fn new(head: NodePtr<'id, T>, token: GhostToken<'id>) -> Self {
        Self { head, token }
    }

    pub fn create<I: IntoIterator<Item = T>>(token: GhostToken<'id>, elements: I) -> Self {
        let mut iter = elements.into_iter();
        let head = Node::new(iter.next().unwrap());
        let mut list = ListWrapper { head, token };
        let mut tail = Arc::clone(&list.head);
        while let Some(e) = iter.next() {
            let node = Node::new(e);
            Node::insert_next(&tail, Arc::clone(&node), &mut list.token);
            tail = node;
        }
        list
    }

    pub fn iter<'a>(&'a self) -> Iter<'id, 'a, T> {
        Iter {
            cur: Some(&self.head),
            token: &self.token,
        }
    }

    pub fn expose_node(&self) -> NodePtr<'id, T> {
        Arc::clone(&self.head)
    }

    pub fn expose_token(&self) -> &GhostToken<'id> {
        &self.token
    }

    pub fn expose_mut_node(&mut self) -> &mut Node<'id, T> {
        self.head.borrow_mut(&mut self.token)
    }
}

impl<'id, T: Debug> Debug for ListWrapper<'id, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let view = Node::view_as_vec(&self.head, &self.token);
        f.write_str(format!("{:?}", view).as_str())
    }
}

mod ownership {
    pub mod data_structure_lib {
        use std::sync::Arc;

        use ghost_cell::{GhostCell, GhostToken};

        // private struct, shouldn't be exposed to users
        struct Rep {
            a: i32,
        }
        type RepPointer<'id> = Arc<GhostCell<'id, Rep>>;

        pub struct S1<'id> {
            data: RepPointer<'id>,
        }

        impl<'id> S1<'id> {
            pub fn new(a: i32) -> Self {
                Self {
                    data: Arc::new(GhostCell::new(Rep { a })),
                }
            }

            /// mixing self' and other's representations is allowed when they
            /// have common brand
            pub fn mix_representations(&mut self, other: &S1<'id>) {
                let other_rep = Arc::clone(&other.data);
                self.data = other_rep;
            }
        }

        pub struct SWithToken<'id> {
            token: GhostToken<'id>,
            data: RepPointer<'id>,
        }

        impl<'id> SWithToken<'id> {
            pub fn new(a: i32, token: GhostToken<'id>) -> Self {
                Self {
                    token,
                    data: Arc::new(GhostCell::new(Rep { a })),
                }
            }

            // Does not compile, lifetimes don't match
            pub fn mix_representations_fails<'id2>(&mut self, other: &SWithToken<'id2>) {
                // let other_rep = Arc::clone(&other.data);
                // self.data = other_rep;
            }
        }
    }
    pub mod client_lib {
        use ghost_cell::GhostToken;

        use super::data_structure_lib::*;

        pub fn mix_representations() {
            let mut s1_1 = S1::new(1);
            let s1_2 = S1::new(2);

            s1_1.mix_representations(&s1_2);
        }

        pub fn try_put_two_structs_in_one_vector() {
            GhostToken::new(|mut token1| {
                GhostToken::new(|mut token2| {
                    let swt1 = SWithToken::new(1, token1);
                    let swt2 = SWithToken::new(2, token2);

                    let mut swts1 = vec![swt1];
                    let mut swts2 = vec![swt2];

                    // does not compile as swt lists have different types (lifetimes in particular):
                    // swts1.append(&mut swts2);
                })
            })
        }

        pub fn mix_representations_fails() {
            GhostToken::new(|mut token1| {
                GhostToken::new(|mut token2| {
                    let mut swt1 = SWithToken::new(1, token1);
                    let swt2 = SWithToken::new(2, token2);

                    swt1.mix_representations_fails(&swt2);
                })
            })
        }

        pub fn run_all_examples() {
            mix_representations();
            mix_representations_fails();
            try_put_two_structs_in_one_vector();
        }
    }
}

mod dllist_client_lib {
    use ghost_cell::GhostToken;

    use crate::{init_list, ListWrapper, Node};

    pub fn list_wrapper_usage() {
        // ListWrapper can store the token that owns its list nodes
        // This allows not passing token everywhere
        GhostToken::new(|mut token| {
            let list = ListWrapper::create(token, [1, 2, 3, 4]);
            println!("{:?}", list);

            for n in list.iter() {
                println!("{}", n);
            }
        });
    }

    pub fn view_as_vec() {
        GhostToken::new(|mut token| {
            let (list, tail) = init_list(&mut token, 5);

            let view = Node::view_as_vec(&list, &token);
            println!("{:?}", view);
        });
    }

    pub fn immutable_incoming_aliases_allowed() {
        GhostToken::new(|mut token| {
            let (list, _tail) = init_list(&mut token, 5);
            let list_wrapper = ListWrapper::new(list, token);

            let token_alias = list_wrapper.expose_token();
            let node_alias = list_wrapper.expose_node();
            let _x = node_alias.borrow(token_alias).data;
        });
    }

    pub fn mutable_incoming_alias_allowed() {
        GhostToken::new(|mut token| {
            let (list, _tail) = init_list(&mut token, 5);
            let mut list_wrapper = ListWrapper::new(list, token);

            let mut_node_ref = list_wrapper.expose_mut_node();
            mut_node_ref.data = 666;
            println!("{:?}", list_wrapper);
        });
    }

    pub fn run_all_examples() {
        list_wrapper_usage();
        view_as_vec();
        immutable_incoming_aliases_allowed();
        mutable_incoming_alias_allowed();
    }
}

fn main() {
    ownership::client_lib::run_all_examples();
    dllist_client_lib::run_all_examples();
}
