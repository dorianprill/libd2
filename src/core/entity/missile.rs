use crate::core::coordinate::Coordinate;
use crate::core::entity::Entity;
use crate::core::update::Update;
use crate::ServerMessage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Missile {
    id: u32,
    class_id: Option<u16>,
    location: Coordinate,
    target: Option<Coordinate>,
    current_frame: Option<u16>,
    owner_type: Option<u8>,
    owner_id: Option<u32>,
    skill_level: Option<u8>,
    pierce_level: Option<u8>,
}

impl Missile {
    pub fn new(id: u32, location: Coordinate) -> Self {
        Self {
            id,
            class_id: None,
            location,
            target: None,
            current_frame: None,
            owner_type: None,
            owner_id: None,
            skill_level: None,
            pierce_level: None,
        }
    }

    pub fn class_id(&self) -> Option<u16> {
        self.class_id
    }

    pub fn set_class_id(&mut self, class_id: u16) {
        self.class_id = Some(class_id);
    }

    pub fn set_location(&mut self, location: Coordinate) {
        self.location = location;
    }

    pub fn target(&self) -> Option<Coordinate> {
        self.target
    }

    pub fn set_target(&mut self, target: Coordinate) {
        self.target = Some(target);
    }

    pub fn current_frame(&self) -> Option<u16> {
        self.current_frame
    }

    pub fn set_current_frame(&mut self, current_frame: u16) {
        self.current_frame = Some(current_frame);
    }

    pub fn owner_type(&self) -> Option<u8> {
        self.owner_type
    }

    pub fn set_owner_type(&mut self, owner_type: u8) {
        self.owner_type = Some(owner_type);
    }

    pub fn owner_id(&self) -> Option<u32> {
        self.owner_id
    }

    pub fn set_owner_id(&mut self, owner_id: u32) {
        self.owner_id = Some(owner_id);
    }

    pub fn skill_level(&self) -> Option<u8> {
        self.skill_level
    }

    pub fn set_skill_level(&mut self, skill_level: u8) {
        self.skill_level = Some(skill_level);
    }

    pub fn pierce_level(&self) -> Option<u8> {
        self.pierce_level
    }

    pub fn set_pierce_level(&mut self, pierce_level: u8) {
        self.pierce_level = Some(pierce_level);
    }
}

impl Update for Missile {
    fn update(&mut self, _msg: ServerMessage) -> bool {
        // TODO match packets here e.g. MercUpdate etc.
        true
    }
}

impl Entity for Missile {
    fn initialized(&self) -> bool {
        true
    }

    fn id(&self) -> u32 {
        self.id
    }

    fn location(&self) -> Coordinate {
        self.location
    }
}
