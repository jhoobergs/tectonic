// Copyright 2018-2022 the Tectonic Project
// Licensed under the MIT License.

#![deny(missing_docs)]

//! Convert Tectonic’s SPX format to HTML.
//!
//! SPX is essentially the same thing as XDV, but we identify it differently to
//! mark that the semantics of the content wil be set up for HTML output.

use std::path::Path;
use tectonic_bridge_core::DriverHooks;
use tectonic_errors::prelude::*;
use tectonic_status_base::StatusBackend;
use tectonic_xdv::{FileType, XdvEvents, XdvParser};

mod emission;
mod fontfamily;
mod fontfile;
mod html;
mod initialization;
mod templating;

use emission::EmittingState;
use initialization::InitializationState;

/// An engine that converts SPX to HTML.
#[derive(Default)]
pub struct Spx2HtmlEngine {}

impl Spx2HtmlEngine {
    /// Process SPX into HTML.
    ///
    /// Because this driver will, in the generic case, produce a tree of HTML
    /// output files that are not going to be used as a basis for any subsequent
    /// engine stages, it outputs directly to disk (via `out_base`) rather than
    /// using the I/O layer. I don't like hardcoding use of the filesystem, but
    /// I don't want to build up some extra abstraction layer right now.
    pub fn process_to_filesystem(
        &mut self,
        hooks: &mut dyn DriverHooks,
        status: &mut dyn StatusBackend,
        spx: &str,
        out_base: &Path,
    ) -> Result<()> {
        let mut input = hooks.io().input_open_name(spx, status).must_exist()?;

        {
            let state = EngineState::new(hooks, status, out_base);
            let state = XdvParser::process_with_seeks(&mut input, state)?;
            state.finished()?;
        }

        let (name, digest_opt) = input.into_name_digest();
        hooks.event_input_closed(name, digest_opt, status);
        Ok(())
    }
}

struct EngineState<'a> {
    common: Common<'a>,
    state: State,
}

struct Common<'a> {
    hooks: &'a mut dyn DriverHooks,
    status: &'a mut dyn StatusBackend,
    out_base: &'a Path,
}

impl<'a> EngineState<'a> {
    pub fn new(
        hooks: &'a mut dyn DriverHooks,
        status: &'a mut dyn StatusBackend,
        out_base: &'a Path,
    ) -> Self {
        Self {
            common: Common {
                hooks,
                status,
                out_base,
            },
            state: State::Initializing(InitializationState::default()),
        }
    }
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum State {
    /// This variant is needed to implement state changes.
    Invalid,
    Initializing(InitializationState),
    Emitting(EmittingState),
}

impl<'a> EngineState<'a> {
    pub fn finished(mut self) -> Result<()> {
        if let State::Emitting(mut s) = self.state {
            s.finished(&mut self.common)?;
        }

        Ok(())
    }

    /// Return true if we're in the initializing phase, but not in the midst of
    /// a multi-step construct like startDefineFontFamily. In such situations,
    /// if we see an event that is associated with the beginning of the actual
    /// content, we should end the initialization phase.
    fn in_endable_init(&self) -> bool {
        match &self.state {
            State::Invalid => false,
            State::Initializing(s) => s.in_endable_init(),
            State::Emitting(_) => false,
        }
    }
}

impl<'a> XdvEvents for EngineState<'a> {
    type Error = Error;

    fn handle_header(&mut self, filetype: FileType, _comment: &[u8]) -> Result<()> {
        if filetype != FileType::Spx {
            bail!("file should be SPX format but got {}", filetype);
        }

        Ok(())
    }

    fn handle_special(&mut self, x: i32, y: i32, contents: &[u8]) -> Result<()> {
        let contents = atry!(std::str::from_utf8(contents); ["could not parse \\special as UTF-8"]);

        // str.split_once() would be nice but it was introduced in 1.52 which is
        // a bit recent for us.

        let mut pieces = contents.splitn(2, ' ');

        let (tdux_command, remainder) = if let Some(p) = pieces.next() {
            if let Some(cmd) = p.strip_prefix("tdux:") {
                (Some(cmd), pieces.next().unwrap_or_default())
            } else {
                (None, contents)
            }
        } else {
            (None, contents)
        };

        // Might we need to end the initialization phase?

        if self.in_endable_init() {
            let end_init = matches!(
                tdux_command.unwrap_or("none"),
                "emit" | "provideFile" | "asp" | "aep" | "cs" | "ce" | "mfs" | "me" | "dt"
            );

            if end_init {
                self.state.ensure_initialized()?;
            }
        }

        // Ready to dispatch.

        match &mut self.state {
            State::Invalid => panic!("invalid spx2html state leaked"),
            State::Initializing(s) => s.handle_special(tdux_command, remainder, &mut self.common),
            State::Emitting(s) => s.handle_special(x, y, tdux_command, remainder, &mut self.common),
        }
    }

    fn handle_text_and_glyphs(
        &mut self,
        font_num: FontNum,
        text: &str,
        _width: i32,
        glyphs: &[u16],
        x: &[i32],
        y: &[i32],
    ) -> Result<()> {
        if self.in_endable_init() {
            self.state.ensure_initialized()?;
        }

        match &mut self.state {
            State::Invalid => panic!("invalid spx2html state leaked"),
            State::Initializing(s) => {
                s.handle_text_and_glyphs(font_num, text, glyphs, x, y, &mut self.common)?
            }
            State::Emitting(s) => {
                s.handle_text_and_glyphs(font_num, text, glyphs, x, y, &mut self.common)?
            }
        }

        Ok(())
    }

    fn handle_define_native_font(
        &mut self,
        name: &str,
        font_num: FontNum,
        size: i32,
        face_index: u32,
        color_rgba: Option<u32>,
        extend: Option<u32>,
        slant: Option<u32>,
        embolden: Option<u32>,
    ) -> Result<(), Self::Error> {
        match &mut self.state {
            State::Invalid => panic!("invalid spx2html state leaked"),
            State::Initializing(s) => s.handle_define_native_font(
                name,
                font_num,
                size,
                face_index,
                color_rgba,
                extend,
                slant,
                embolden,
                &mut self.common,
            ),
            _ => Ok(()),
        }
    }

    fn handle_glyph_run(
        &mut self,
        font_num: FontNum,
        glyphs: &[u16],
        x: &[i32],
        y: &[i32],
    ) -> Result<(), Self::Error> {
        self.state.ensure_initialized()?;

        match &mut self.state {
            State::Invalid => panic!("invalid spx2html state leaked"),
            State::Initializing(_) => unreachable!(),
            State::Emitting(s) => s.handle_glyph_run(font_num, glyphs, x, y, &mut self.common),
        }
    }
}

impl State {
    fn ensure_initialized(&mut self) -> Result<()> {
        // Is this the least-bad way to do this??
        let mut work = std::mem::replace(self, State::Invalid);

        if let State::Initializing(s) = work {
            work = State::Emitting(s.initialization_finished()?);
        }

        std::mem::swap(self, &mut work);
        Ok(())
    }
}

type FixedPoint = i32;
type FontNum = i32;
