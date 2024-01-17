mod dllist_lib {
    use std::sync::{Arc, Weak};

    use qcell::{TCell, TCellOwner};

    pub struct Node<T, Brand> {
        pub data: T,
        next: Option<NodePtr<T, Brand>>,
        prev: Option<WeakNodePtr<T, Brand>>,
    }
    pub type NodePtr<T, Brand> = Arc<TCell<Brand, Node<T, Brand>>>;
    pub type WeakNodePtr<T, Brand> = Weak<TCell<Brand, Node<T, Brand>>>;

    impl<T, Brand> Node<T, Brand> {
        pub fn new(value: T) -> NodePtr<T, Brand> {
            Arc::new(TCell::new(Self {
                data: value,
                prev: None,
                next: None,
            }))
        }

        /// Unlink the nodes adjacent to `node`. The node will have `next` and `prev` be `None` after this.
        pub fn remove(node: &NodePtr<T, Brand>, token: &mut TCellOwner<Brand>) {
            let node = node.rw(token);

            let old_prev: Option<NodePtr<T, Brand>> = node.prev.take().and_then(|p| p.upgrade());
            let old_next: Option<NodePtr<T, Brand>> = node.next.take();
            if let Some(old_next) = &old_next {
                old_next.rw(token).prev = old_prev.as_ref().map(|p| Arc::downgrade(&p));
            }
            if let Some(old_prev) = &old_prev {
                old_prev.rw(token).next = old_next;
            }
        }

        /// Insert `node2` right after `node1` in the list.
        pub fn insert_next(
            node1: &NodePtr<T, Brand>,
            node2: NodePtr<T, Brand>,
            token: &mut TCellOwner<Brand>,
        ) {
            Self::remove(&node2, token);

            let node1_old_next: Option<NodePtr<T, Brand>> = node1.rw(token).next.take();
            if let Some(node1_old_next) = &node1_old_next {
                node1_old_next.rw(token).prev = Some(Arc::downgrade(&node2));
            }

            let node2_inner: &mut Node<T, Brand> = node2.rw(token);
            node2_inner.prev = Some(Arc::downgrade(node1));
            node2_inner.next = node1_old_next;

            node1.rw(token).next = Some(node2);
        }

        pub fn from_iter<I: IntoIterator<Item = T>>(
            token: &mut TCellOwner<Brand>,
            elements: I,
        ) -> Option<NodePtr<T, Brand>> {
            let mut iter = elements.into_iter();
            let first_element = iter.next()?;
            let head = Node::new(first_element);
            let mut tail = Arc::clone(&head);
            while let Some(e) = iter.next() {
                let node = Node::new(e);
                Node::insert_next(&tail, Arc::clone(&node), token);
                tail = node;
            }
            Option::Some(head)
        }

        pub fn view_as_vec<'a>(
            head: Option<&'a NodePtr<T, Brand>>,
            token: &'a TCellOwner<Brand>,
        ) -> Vec<&'a T> {
            let mut cur: Option<&NodePtr<T, Brand>> = head;
            let mut v: Vec<&'a T> = vec![];
            while let Some(node) = cur {
                v.push(&node.ro(token).data);
                cur = (&node.ro(token).next).as_ref();
            }
            v
        }

        pub fn next(&self) -> Option<&NodePtr<T, Brand>> {
            self.next.as_ref()
        }
    }
}

mod client_lib {
    use std::sync::Arc;

    use qcell::TCellOwner;

    use crate::dllist_lib::{Node, NodePtr};

    pub fn simple_usage() {
        struct Brand;
        let mut token = TCellOwner::<Brand>::new();
        let list1 = Node::from_iter(&mut token, [1, 2, 3]);
        println!("{:?}", Node::view_as_vec(list1.as_ref(), &token));
    }

    pub fn unique_owner_restriction() {
        struct Brand;
        let mut token1 = TCellOwner::<Brand>::new();
        let list1 = Node::from_iter(&mut token1, [1, 2, 3]);
        // will panic:
        // let token2 = TCellOwner::<Brand>::new();
    }

    pub fn static_owner_check() {
        struct Brand;
        let mut token1 = TCellOwner::<Brand>::new();
        let list1 = Node::from_iter(&mut token1, [1, 2, 3]);

        // does not compile
        // struct Brand2;
        // let token2 = TCellOwner::<Brand2>::new();
        // println!("{:?}", list1.map(|l| l.ro(&token2).data))
    }

    pub fn two_simultaneous_borrows() {
        struct Brand;
        let mut token = TCellOwner::<Brand>::new();
        let first = Node::from_iter(&mut token, [1, 2]).unwrap();
        let second = Arc::clone(first.ro(&token).next().unwrap());
        let (first_ref, second_ref) = token.rw2(&first, &second);

        first_ref.data = 61;
        second_ref.data = 62;
        println!("{:?}", Node::view_as_vec(Option::Some(&first), &token));
    }

    pub fn two_simultaneous_borrows_panic() {
        // 2 simultaneous borrows panic as references point to one cell
        struct Brand;
        let mut token = TCellOwner::<Brand>::new();
        let first = Node::from_iter(&mut token, [1]).unwrap();
        let second = Arc::clone(&first);
        // panics:
        // let (first_ref, second_ref) = token.rw2(&first, &second);
    }

    pub fn two_structs_in_one_vector_fail() {
        trait Brand {

        }

        struct Brand1;
        let mut token1 = TCellOwner::<Brand1>::new();
        let first = Node::from_iter(&mut token1, [1, 2, 3]);

        struct Brand2;
        let mut token2 = TCellOwner::<Brand2>::new();
        let second = Node::from_iter(&mut token2, [1, 2, 3]);

        // does not compile:
        // struct MultipleListsContainer<T> {
        //     lists: Vec<Option<NodePtr<T, dyn Brand>>>,
        // }
    }

    pub fn run_all_examples() {
        simple_usage();
        unique_owner_restriction();
        static_owner_check();
        two_simultaneous_borrows();
        two_simultaneous_borrows_panic();
        two_structs_in_one_vector_fail();
    }
}

fn main() {
    client_lib::run_all_examples();
}
