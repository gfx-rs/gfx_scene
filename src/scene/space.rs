use std::marker::PhantomData;
use cgmath::{BaseFloat, Transform, Transform3};

type IdType = u32;
pub struct Id<T>(IdType, PhantomData<T>);

impl<T> Copy for Id<T> {}


#[derive(Copy)]
pub enum Parent<T> {
    None,
    Domestic(Id<Node<T>>),
    Foreign(Id<Skeleton<T>>, Id<Bone<T>>),
}

pub struct Node<T> {
    pub name : String,
    parent: Parent<T>,
    pub local: T,
    pub world: T,
}

pub struct Bone<T> {
    pub name : String,
    parent: Option<Id<Bone<T>>>,
    pub local: T,
    pub world: T,
    _bind_pose: T,
    _bind_pose_root_inverse: T,
}

pub struct Skeleton<T> {
    pub name: String,
    node: Id<Node<T>>,
    bones: Vec<Bone<T>>,
}

pub struct World<S, T> {
    nodes: Vec<Node<T>>,
    skeletons: Vec<Skeleton<T>>,
    phantom: PhantomData<S>,
}

impl<S: BaseFloat, T: Transform3<S> + Clone> World<S, T> {
    pub fn new() -> World<S, T> {
        World {
            nodes: Vec::new(),
            skeletons: Vec::new(),
            phantom: PhantomData,
        }
    }

    pub fn get_node(&self, id: Id<Node<T>>) -> &Node<T> {
        let Id(nid, _) = id;
        self.nodes.get(nid as usize).unwrap()
    }

    pub fn mut_node(&mut self, id: Id<Node<T>>) -> &mut Node<T> {
        let Id(nid, _) = id;
        self.nodes.get_mut(nid as usize).unwrap()
    }

    pub fn find_node(&self, name: &str) -> Option<Id<Node<T>>> {
        self.nodes.iter().position(|n| n.name == name)
                         .map(|i| Id(i as IdType, PhantomData))
    }

    pub fn add_node(&mut self, name: String, parent: Parent<T>, local: T)
                    -> Id<Node<T>> {
        //TODO: check that parent is valid
        let nid = Id(self.nodes.len() as IdType, PhantomData);
        self.nodes.push(Node {
            name: name,
            parent: parent,
            local: local,
            world: Transform::identity(),
        });
        nid
    }

    pub fn update(&mut self) {
        for i in 0.. self.nodes.len() {
            let (left, right) = self.nodes.split_at_mut(i);
            let n = &mut right[0];
            n.world = match n.parent {
                Parent::None => n.local.clone(),
                Parent::Domestic(Id(pid, _)) => {
                    assert!((pid as usize) < i);
                    left[pid as usize].world.concat(&n.local)
                },
                Parent::Foreign(Id(sid, _), Id(bid, _)) => {
                    self.skeletons[sid as usize]
                        .bones[bid as usize]
                        .world.concat(&n.local)
                },
            };
        }

        //TODO: refactor to avoid a possible lag, caused by bone parenting

        for s in self.skeletons.iter_mut() {
            let Id(nid, _) = s.node;
            let world = &self.nodes[nid as usize].world;
            for i in 0.. s.bones.len() {
                let (left, right) = s.bones.split_at_mut(i);
                let b = &mut right[0];
                let base = match b.parent {
                    Some(Id(bid, _)) => {
                        assert!((bid as usize) < i);
                        &left[bid as usize].world
                    },
                    None => world
                };
                b.world = base.concat(&b.local);
            }
        }
    }
}
