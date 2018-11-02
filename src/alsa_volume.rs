// This file is part of perspektiv, a userspace daemon for graphically reporting
// system events.
// Copyright © 2018  Henrik Laxhuber <henrik@laxhuber.com>
//
// perspektiv is free software: you can redistribute it and/or modify it under
// the terms of the GNU General Public License, version 3, as published by the
// Free Software Foundation.
//
// This program is distributed in the hope that it will be useful, but WITHOUT ANY
// WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A
// PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with
// this program. If not, see <https://www.gnu.org/licenses/>.

// Initial pointer in the right direction from
// https://stackoverflow.com/questions/34936783/watch-for-volume-changes-in-alsa-pulseaudio

extern crate alsa;

use std::mem;

use self::alsa::mixer::{Mixer, Selem, SelemChannelId, SelemId};
use self::alsa::poll::*;
use libc::pollfd;

use subscribable;
use subscribable::Subscribable;
use ui;

const SND_CTL_TLV_DB_GAIN_MUTE: i64 = -9_999_999;
const MAX_LINEAR_DB_SCALE: i64 = 24;

struct Card {
    name: String,
    ctl: alsa::Ctl,
    hwid: String,
    fd_i: usize,
    fd_n: usize,
    volume: f64,
}

impl Card {
    fn new(alsa_card: alsa::Card, poll_fds: &mut Vec<pollfd>) -> alsa::Result<Self> {
        let name = alsa_card.get_name().unwrap_or("<unknown name>".to_string());
        let ctl = alsa::Ctl::from_card(&alsa_card, false)?;
        let hwid = format!("hw:{}", alsa_card.get_index());

        let mut fds = PollDescriptors::get(&ctl)?;

        // fd_i is the index where pollfds for this card start. fd_n is the
        // number of pollfds for this card. This should really only ever be
        // one, so this whole process could be simplified a lot. The current
        // behaviour is somewhat unsafe if and only if elements from poll_fd
        // are ever deleted.
        // This hacky behaviour is used to track which pollfds describe
        // which card such that card.ctl.revents() won't fail on invalid
        // pollfds, while also storing all pollfds in an array for
        // libc::poll to be happy.
        let card = Card {
            name,
            ctl,
            hwid,
            fd_i: poll_fds.len(),
            fd_n: fds.len(),
            volume: -2.0,
        };

        // verify that we can get volume, else yield error.
        card.get_master()?.get_volume()?;
        // it works, subscribe and return
        card.ctl.subscribe_events(true)?;
        poll_fds.append(&mut fds);
        Ok(card)
    }

    // FIXME: Need to get a new mixer every time the volume changes for some
    // reason. This is a bit awkward and seems unnecessary.
    fn get_master<'a>(&self) -> alsa::Result<Master<'a>> {
        let mixer = Mixer::new(&self.hwid, false)?;

        unsafe {
            // Transmute because the lifetime of selem is not just the
            // lifetime of the reference to the mixer, but that of the mixer
            // itself, which is 'a. This is "safe" since Mixer may be moved
            // in rust code without affecting the underlying FFI code (it's
            // effectively a box).
            let selem = mem::transmute::<Selem, Selem<'a>>(match mixer
                .find_selem(&SelemId::new("Master", 0))
            {
                Some(selem) => Ok(selem),
                None => Err(alsa::Error::new("find_selem", -1)),
            }?);

            Ok(Master(selem, mixer))
        }
    }
}

struct Master<'a>(Selem<'a>, Mixer);
impl<'a> Master<'a> {
    fn get_mute(&self) -> alsa::Result<bool> {
        let selem: &Selem = &self.0;

        Ok(selem.get_playback_switch(SelemChannelId::Unknown)? == 0)
    }

    fn get_volume(&self) -> alsa::Result<f64> {
        let selem: &Selem = &self.0;

        let range = selem.get_playback_db_range();
        let range = ((range.0).0, (range.1).0); // get interior i64 mB value
        let mut volume: f64 = selem.get_playback_vol_db(SelemChannelId::Unknown)?.0 as f64;

        // The following performs alsamixer-style volume mapping, as seen in
        // https://github.com/bear24rw/alsa-utils/blob/master/alsamixer/volume_mapping.c

        // linear normalisation if the volume range is small
        if range.1 - range.0 <= MAX_LINEAR_DB_SCALE * 100 {
            volume = (volume - range.0 as f64) / (range.1 - range.0) as f64;
        } else {
            // else, do exponential normalisation
            volume = 10_f64.powf((volume - range.1 as f64) / 6_000.0);
            if range.0 != SND_CTL_TLV_DB_GAIN_MUTE {
                let range: (f64, f64) = (
                    10_f64.powf((range.0 - range.1) as f64 / 6_000.0),
                    range.1 as f64,
                );
                volume = (volume - range.0) / (1.0 - range.0);
            }
        }

        Ok(volume)
    }
}

pub struct Subscription();
impl Subscribable for Subscription {
    type Params = ();

    fn poll_factory(
        _params: Self::Params,
    ) -> Result<Box<subscribable::PollFn>, String> {
        let mut poll_fds: Vec<pollfd> = Vec::new();
        let mut cards: Vec<Card> = alsa::card::Iter::new()
            .filter_map(|card| match Card::new(card.unwrap(), &mut poll_fds) {
                Ok(card) => Some(card),
                Err(_) => None,
            })
            .collect();

        err_if!(cards.len() == 0, "Failed to find any sound cards with a master volume.".to_string());

        Ok(Box::new(move || {
            loop {
                poll(&mut poll_fds, -1).unwrap();

                for card in &mut cards {
                    let flags = card
                        .ctl
                        .revents(&poll_fds[card.fd_i..card.fd_i + card.fd_n])
                        .unwrap();
                    if !flags.is_empty() {
                        if flags == POLLIN {
                            card.ctl.read().unwrap();
                            let master = card.get_master().unwrap();
                            let muted = master.get_mute().unwrap();
                            let volume = master.get_volume().unwrap();

                            if muted && card.volume != -1.0 {
                                card.volume = -1.0;
                                return Ok(ui::ShowBool("", "Muted"));
                            } else if !muted && volume != card.volume {
                                card.volume = volume;
                                return Ok(ui::ShowPercent("", card.volume));
                            }
                        } else {
                            return Err(subscribable::Error::from(
                                format!(
                                    "Got unexpected poll flags for {}: {:#?}",
                                    card.name, flags
                                )
                            ));
                        }
                    }
                }
                // if no event matched the criterea, loop to poll again
            } // while true
        }))
    }
}
