// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

use super::Backend;
use crate::{
    backend_proto as pb,
    latex::{extract_latex, extract_latex_expanding_clozes, ExtractedLatex},
    notetype::CardTemplateSchema11,
    prelude::*,
    text::{extract_av_tags, strip_av_tags, AVTag},
};
pub(super) use pb::cardrendering_service::Service as CardRenderingService;

impl CardRenderingService for Backend {
    fn extract_av_tags(&self, input: pb::ExtractAvTagsIn) -> Result<pb::ExtractAvTagsOut> {
        let (text, tags) = extract_av_tags(&input.text, input.question_side);
        let pt_tags = tags
            .into_iter()
            .map(|avtag| match avtag {
                AVTag::SoundOrVideo(file) => pb::AvTag {
                    value: Some(pb::av_tag::Value::SoundOrVideo(file)),
                },
                AVTag::TextToSpeech {
                    field_text,
                    lang,
                    voices,
                    other_args,
                    speed,
                } => pb::AvTag {
                    value: Some(pb::av_tag::Value::Tts(pb::TtsTag {
                        field_text,
                        lang,
                        voices,
                        other_args,
                        speed,
                    })),
                },
            })
            .collect();

        Ok(pb::ExtractAvTagsOut {
            text: text.into(),
            av_tags: pt_tags,
        })
    }

    fn extract_latex(&self, input: pb::ExtractLatexIn) -> Result<pb::ExtractLatexOut> {
        let func = if input.expand_clozes {
            extract_latex_expanding_clozes
        } else {
            extract_latex
        };
        let (text, extracted) = func(&input.text, input.svg);

        Ok(pb::ExtractLatexOut {
            text,
            latex: extracted
                .into_iter()
                .map(|e: ExtractedLatex| pb::ExtractedLatex {
                    filename: e.fname,
                    latex_body: e.latex,
                })
                .collect(),
        })
    }

    fn get_empty_cards(&self, _input: pb::Empty) -> Result<pb::EmptyCardsReport> {
        self.with_col(|col| {
            let mut empty = col.empty_cards()?;
            let report = col.empty_cards_report(&mut empty)?;

            let mut outnotes = vec![];
            for (_ntid, notes) in empty {
                outnotes.extend(notes.into_iter().map(|e| {
                    pb::empty_cards_report::NoteWithEmptyCards {
                        note_id: e.nid.0,
                        will_delete_note: e.empty.len() == e.current_count,
                        card_ids: e.empty.into_iter().map(|(_ord, id)| id.0).collect(),
                    }
                }))
            }
            Ok(pb::EmptyCardsReport {
                report,
                notes: outnotes,
            })
        })
    }

    fn render_existing_card(&self, input: pb::RenderExistingCardIn) -> Result<pb::RenderCardOut> {
        self.with_col(|col| {
            col.render_existing_card(CardID(input.card_id), input.browser)
                .map(Into::into)
        })
    }

    fn render_uncommitted_card(
        &self,
        input: pb::RenderUncommittedCardIn,
    ) -> Result<pb::RenderCardOut> {
        let schema11: CardTemplateSchema11 = serde_json::from_slice(&input.template)?;
        let template = schema11.into();
        let mut note = input
            .note
            .ok_or_else(|| AnkiError::invalid_input("missing note"))?
            .into();
        let ord = input.card_ord as u16;
        let fill_empty = input.fill_empty;
        self.with_col(|col| {
            col.render_uncommitted_card(&mut note, &template, ord, fill_empty)
                .map(Into::into)
        })
    }

    fn strip_av_tags(&self, input: pb::String) -> Result<pb::String> {
        Ok(pb::String {
            val: strip_av_tags(&input.val).into(),
        })
    }
}