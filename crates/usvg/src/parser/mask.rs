// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::sync::Arc;

use svgtypes::{Length, LengthUnit as Unit};

use super::svgtree::{AId, EId, SvgNode};
use super::{converter, OptionLog};
use crate::{Group, Mask, MaskType, NonEmptyString, NonZeroRect, Units};

pub(crate) fn convert(
    node: SvgNode,
    state: &converter::State,
    cache: &mut converter::Cache,
) -> Option<Arc<Mask>> {
    // A `mask` attribute must reference a `mask` element.
    if node.tag_name() != Some(EId::Mask) {
        return None;
    }

    // Check if this element was already converted.
    if let Some(mask) = cache.masks.get(node.element_id()) {
        return Some(mask.clone());
    }

    let id = NonEmptyString::new(node.element_id().to_string())?;

    let units = node
        .attribute(AId::MaskUnits)
        .unwrap_or(Units::ObjectBoundingBox);

    let content_units = node
        .attribute(AId::MaskContentUnits)
        .unwrap_or(Units::UserSpaceOnUse);

    let rect = NonZeroRect::from_xywh(
        node.convert_length(AId::X, units, state, Length::new(-10.0, Unit::Percent)),
        node.convert_length(AId::Y, units, state, Length::new(-10.0, Unit::Percent)),
        node.convert_length(AId::Width, units, state, Length::new(120.0, Unit::Percent)),
        node.convert_length(AId::Height, units, state, Length::new(120.0, Unit::Percent)),
    );
    let rect =
        rect.log_none(|| log::warn!("Mask '{}' has an invalid size. Skipped.", node.element_id()))?;

    // Resolve linked mask.
    let mut mask = None;
    if let Some(link) = node.attribute::<SvgNode>(AId::Mask) {
        mask = convert(link, state, cache);

        // Linked `mask` must be valid.
        if mask.is_none() {
            return None;
        }
    }

    let kind = if node.attribute(AId::MaskType) == Some("alpha") {
        MaskType::Alpha
    } else {
        MaskType::Luminance
    };

    let mut mask = Mask {
        id,
        units,
        content_units,
        rect,
        kind,
        mask,
        root: Group::empty(),
    };

    converter::convert_children(node, state, cache, &mut mask.root);

    if mask.root.has_children() {
        mask.root.calculate_bounding_boxes();
        let mask = Arc::new(mask);
        cache
            .masks
            .insert(node.element_id().to_string(), mask.clone());
        Some(mask)
    } else {
        // A mask without children is invalid.
        None
    }
}