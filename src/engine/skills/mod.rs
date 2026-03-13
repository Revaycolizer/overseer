// src/skills.rs

pub struct Skill {
    pub name: &'static str,
    pub content: &'static str,
}

pub const BROWSER_SKILL: Skill = Skill {
    name: "Browser Navigation",
    content: include_str!("browser.md"),
};

pub const DOCKER_SKILL: Skill = Skill {
    name: "Docker Management",
    content: include_str!("docker.md"),
};

pub const FRONTEND_SKILL: Skill = Skill {
    name: "Frontend Development",
    content: include_str!("frontend.md"),
};

pub const BACKEND_SKILL: Skill = Skill {
    name: "Backend Development",
    content: include_str!("backend.md"),
};

pub fn get_all_skills() -> Vec<Skill> {
    vec![
        BROWSER_SKILL,
        DOCKER_SKILL,
        FRONTEND_SKILL,
        BACKEND_SKILL,
    ]
}
