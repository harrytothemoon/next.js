pub(crate) mod app_client_references_chunks;
pub(crate) mod app_client_shared_chunks;
pub(crate) mod app_entry;
pub(crate) mod app_favicon_entry;
pub(crate) mod app_page_entry;
pub(crate) mod app_route_entry;
pub(crate) mod unsupported_dynamic_metadata_issue;

use std::{
    fmt::{Display, Formatter, Write},
    ops::Deref,
};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use turbo_tasks::{trace::TraceRawVcs, TaskInput};

pub use crate::next_app::{
    app_client_references_chunks::{
        get_app_client_references_chunks, ClientReferenceChunks, ClientReferencesChunks,
    },
    app_client_shared_chunks::get_app_client_shared_chunks,
    app_entry::AppEntry,
    app_favicon_entry::get_app_route_favicon_entry,
    app_page_entry::get_app_page_entry,
    app_route_entry::get_app_route_entry,
    unsupported_dynamic_metadata_issue::UnsupportedDynamicMetadataIssue,
};

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq, TaskInput, TraceRawVcs)]
pub enum PageSegment {
    Static(String),
    Dynamic(String),
    CatchAll(String),
    OptionalCatchAll(String),
    Group(String),
    Parallel(String),
    PageType(PageType),
}

impl PageSegment {
    pub fn parse(segment: &str) -> Result<Self> {
        if segment.is_empty() {
            bail!("empty segments are not allowed");
        }

        if segment.contains('/') {
            bail!("slashes are not allowed in segments");
        }

        if let Some(s) = segment.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
            return Ok(PageSegment::Group(s.to_string()));
        }

        if let Some(s) = segment.strip_prefix('@') {
            return Ok(PageSegment::Parallel(s.to_string()));
        }

        if let Some(s) = segment
            .strip_prefix("[[...")
            .and_then(|s| s.strip_suffix("]]"))
        {
            return Ok(PageSegment::OptionalCatchAll(s.to_string()));
        }

        if let Some(s) = segment
            .strip_prefix("[...")
            .and_then(|s| s.strip_suffix(']'))
        {
            return Ok(PageSegment::CatchAll(s.to_string()));
        }

        if let Some(s) = segment.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            return Ok(PageSegment::Dynamic(s.to_string()));
        }

        Ok(PageSegment::Static(segment.to_string()))
    }
}

impl Display for PageSegment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PageSegment::Static(s) => f.write_str(s),
            PageSegment::Dynamic(s) => {
                f.write_char('[')?;
                f.write_str(s)?;
                f.write_char(']')
            }
            PageSegment::CatchAll(s) => {
                f.write_str("[...")?;
                f.write_str(s)?;
                f.write_char(']')
            }
            PageSegment::OptionalCatchAll(s) => {
                f.write_str("[[...")?;
                f.write_str(s)?;
                f.write_str("]]")
            }
            PageSegment::Group(s) => {
                f.write_char('(')?;
                f.write_str(s)?;
                f.write_char(')')
            }
            PageSegment::Parallel(s) => {
                f.write_char('@')?;
                f.write_str(s)
            }
            PageSegment::PageType(s) => Display::fmt(s, f),
        }
    }
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq, TaskInput, TraceRawVcs)]
pub enum PageType {
    Page,
    Route,
}

impl Display for PageType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PageType::Page => "page",
            PageType::Route => "route",
        })
    }
}

/// Describes the pathname including all internal modifiers such as
/// intercepting routes, parallel routes and route/page suffixes that are not
/// part of the pathname.
#[derive(
    Clone, Debug, Hash, PartialEq, Eq, Default, Serialize, Deserialize, TaskInput, TraceRawVcs,
)]
pub struct AppPage(pub Vec<PageSegment>);

impl AppPage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, segment: PageSegment) -> Result<()> {
        if matches!(
            self.0.last(),
            Some(PageSegment::CatchAll(..) | PageSegment::OptionalCatchAll(..))
        ) && !matches!(segment, PageSegment::PageType(..))
        {
            bail!(
                "Invalid segment {}, catch all segment must be the last segment",
                segment
            )
        }

        self.0.push(segment);
        Ok(())
    }

    pub fn push_str(&mut self, segment: &str) -> Result<()> {
        if segment.is_empty() {
            return Ok(());
        }

        self.push(PageSegment::parse(segment)?)
    }

    pub fn clone_push(&self, segment: PageSegment) -> Result<Self> {
        let mut cloned = self.clone();
        cloned.push(segment)?;
        Ok(cloned)
    }

    pub fn clone_push_str(&self, segment: &str) -> Result<Self> {
        let mut cloned = self.clone();
        cloned.push_str(segment)?;
        Ok(cloned)
    }

    pub fn parse(page: &str) -> Result<Self> {
        let mut app_page = Self::new();

        for segment in page.split('/') {
            app_page.push_str(segment)?;
        }

        Ok(app_page)
    }
}

impl Display for AppPage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            return f.write_char('/');
        }

        for segment in &self.0 {
            f.write_char('/')?;
            Display::fmt(segment, f)?;
        }

        Ok(())
    }
}

impl Deref for AppPage {
    type Target = [PageSegment];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq, TaskInput, TraceRawVcs)]
pub enum PathSegment {
    Static(String),
    Dynamic(String),
    CatchAll(String),
    OptionalCatchAll(String),
}

impl Display for PathSegment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PathSegment::Static(s) => f.write_str(s),
            PathSegment::Dynamic(s) => {
                f.write_char('[')?;
                f.write_str(s)?;
                f.write_char(']')
            }
            PathSegment::CatchAll(s) => {
                f.write_str("[...")?;
                f.write_str(s)?;
                f.write_char(']')
            }
            PathSegment::OptionalCatchAll(s) => {
                f.write_str("[[...")?;
                f.write_str(s)?;
                f.write_str("]]")
            }
        }
    }
}

/// The pathname (including dynamic placeholders) for a route to resolve.
#[derive(
    Clone, Debug, Hash, PartialEq, Eq, Default, Serialize, Deserialize, TaskInput, TraceRawVcs,
)]
pub struct AppPath(pub Vec<PathSegment>);

impl Deref for AppPath {
    type Target = [PathSegment];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for AppPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            return f.write_char('/');
        }

        for segment in &self.0 {
            f.write_char('/')?;
            Display::fmt(segment, f)?;
        }

        Ok(())
    }
}

impl From<AppPage> for AppPath {
    fn from(value: AppPage) -> Self {
        AppPath(
            value
                .0
                .into_iter()
                .filter_map(|segment| match segment {
                    PageSegment::Static(s) => Some(PathSegment::Static(s)),
                    PageSegment::Dynamic(s) => Some(PathSegment::Dynamic(s)),
                    PageSegment::CatchAll(s) => Some(PathSegment::CatchAll(s)),
                    PageSegment::OptionalCatchAll(s) => Some(PathSegment::OptionalCatchAll(s)),
                    _ => None,
                })
                .collect(),
        )
    }
}
