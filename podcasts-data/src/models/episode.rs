// episode.rs
//
// Copyright 2017 Jordan Petridis <jpetridis@gnome.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::prelude::*;

use diesel::prelude::*;
use diesel::SaveChangesDsl;

use crate::database::connection;
use crate::errors::DataError;
use crate::models::{Save, Show};
use crate::schema::episodes;

#[derive(Queryable, Identifiable, AsChangeset, Associations, PartialEq)]
#[diesel(table_name = episodes)]
#[diesel(treat_none_as_null = true)]
#[diesel(primary_key(title, show_id))]
#[diesel(belongs_to(Show, foreign_key = show_id))]
#[derive(Debug, Clone)]
/// Diesel Model of the episode table.
pub struct Episode {
    rowid: i32,
    title: String,
    uri: Option<String>,
    local_uri: Option<String>,
    description: Option<String>,
    epoch: i32,
    length: Option<i32>,
    duration: Option<i32>,
    play_position: i32,
    guid: Option<String>,
    played: Option<i32>,
    show_id: i32,
}

impl Save<Episode> for Episode {
    type Error = DataError;

    /// Helper method to easily save/"sync" current state of self to the
    /// Database.
    fn save(&self) -> Result<Episode, Self::Error> {
        let db = connection();
        let mut tempdb = db.get()?;

        self.save_changes::<Episode>(&mut tempdb)
            .map_err(From::from)
    }
}

impl Episode {
    /// Get the value of the sqlite's `ROW_ID`
    pub fn rowid(&self) -> i32 {
        self.rowid
    }

    /// Get the value of the `title` field.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Get the value of the `uri`.
    ///
    /// Represents the url(usually) that the media file will be located at.
    pub fn uri(&self) -> Option<&str> {
        self.uri.as_deref()
    }

    /// Get the value of the `local_uri`.
    ///
    /// Represents the local uri,usually filesystem path,
    /// that the media file will be located at.
    pub fn local_uri(&self) -> Option<&str> {
        self.local_uri.as_deref()
    }

    /// Get the `description`.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Get the Episode's `guid`.
    pub fn guid(&self) -> Option<&str> {
        self.guid.as_deref()
    }

    /// Get the `epoch` value.
    ///
    /// Retrieved from the rss Item publish date.
    /// Value is set to Utc whenever possible.
    pub fn epoch(&self) -> i32 {
        self.epoch
    }

    /// Get the `length`.
    ///
    /// The number represents the size of the file in bytes.
    pub fn length(&self) -> Option<i32> {
        self.length
    }

    /// Get the `duration` value.
    ///
    /// The number represents the duration of the item/episode in seconds.
    pub fn duration(&self) -> Option<i32> {
        self.duration
    }

    /// Epoch representation of the last time the episode was played.
    ///
    /// None/Null for unplayed.
    pub fn played(&self) -> Option<i32> {
        self.played
    }

    /// `Show` table foreign key.
    pub fn show_id(&self) -> i32 {
        self.show_id
    }

    /// Get play_position
    ///
    /// The number represents the number of seconds played in the episode.
    /// 0 means the episode was either not played or continued to play to the end.
    pub fn play_position(&self) -> i32 {
        self.play_position
    }
}

#[derive(Queryable, AsChangeset, PartialEq)]
#[diesel(table_name = episodes)]
#[diesel(treat_none_as_null = true)]
#[diesel(primary_key(title, show_id))]
#[derive(Debug, Clone)]
/// Diesel Model to be used for constructing `EpisodeWidgets`.
pub struct EpisodeWidgetModel {
    rowid: i32,
    title: String,
    uri: Option<String>,
    local_uri: Option<String>,
    epoch: i32,
    length: Option<i32>,
    duration: Option<i32>,
    played: Option<i32>,
    play_position: i32,
    show_id: i32,
}

impl From<Episode> for EpisodeWidgetModel {
    fn from(e: Episode) -> EpisodeWidgetModel {
        EpisodeWidgetModel {
            rowid: e.rowid,
            title: e.title,
            uri: e.uri,
            local_uri: e.local_uri,
            epoch: e.epoch,
            length: e.length,
            duration: e.duration,
            played: e.played,
            play_position: e.play_position,
            show_id: e.show_id,
        }
    }
}

impl Save<usize> for EpisodeWidgetModel {
    type Error = DataError;

    /// Helper method to easily save/"sync" current state of self to the
    /// Database.
    fn save(&self) -> Result<usize, Self::Error> {
        use crate::schema::episodes::dsl::*;

        let db = connection();
        let mut tempdb = db.get()?;

        diesel::update(episodes.filter(rowid.eq(self.rowid)))
            .set(self)
            .execute(&mut tempdb)
            .map_err(From::from)
    }
}

impl EpisodeWidgetModel {
    /// Get the value of the sqlite's `ROW_ID`
    pub fn rowid(&self) -> i32 {
        self.rowid
    }

    /// Get the value of the `title` field.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Get the value of the `uri`.
    ///
    /// Represents the url(usually) that the media file will be located at.
    pub fn uri(&self) -> Option<&str> {
        self.uri.as_deref()
    }

    /// Get the value of the `local_uri`.
    ///
    /// Represents the local uri,usually filesystem path,
    /// that the media file will be located at.
    pub fn local_uri(&self) -> Option<&str> {
        self.local_uri.as_deref()
    }

    /// Set the `local_uri`.
    pub fn set_local_uri(&mut self, value: Option<&str>) {
        self.local_uri = value.map(|x| x.to_string());
    }

    /// Get the `epoch` value.
    ///
    /// Retrieved from the rss Item publish date.
    /// Value is set to Utc whenever possible.
    pub fn epoch(&self) -> i32 {
        self.epoch
    }

    /// Get the `length`.
    ///
    /// The number represents the size of the file in bytes.
    pub fn length(&self) -> Option<i32> {
        self.length
    }

    /// Set the `length`.
    pub fn set_length(&mut self, value: Option<i32>) {
        self.length = value;
    }

    /// Get the `duration` value.
    ///
    /// The number represents the duration of the item/episode in seconds.
    pub fn duration(&self) -> Option<i32> {
        self.duration
    }

    /// Epoch representation of the last time the episode was played.
    ///
    /// None/Null for unplayed.
    pub fn played(&self) -> Option<i32> {
        self.played
    }

    /// Set the `played` value.
    fn set_played(&mut self, value: Option<i32>) {
        self.played = value;
    }

    /// `Show` table foreign key.
    pub fn show_id(&self) -> i32 {
        self.show_id
    }

    /// Sets the `played` value with the current `epoch` timestamp and save it.
    pub fn set_played_now(&mut self) -> Result<(), DataError> {
        let epoch = Utc::now().timestamp() as i32;
        self.set_played(Some(epoch));
        self.save().map(|_| ())
    }

    /// Get play_position
    ///
    /// The number represents the number of seconds played in the episode.
    /// `0` means the episode was either not played or continued to play to the end.
    pub fn play_position(&self) -> i32 {
        self.play_position
    }

    /// Sets `play_position` and saves the record.
    pub fn set_play_position(&mut self, seconds: i32) -> Result<(), DataError> {
        self.play_position = seconds;
        self.save().map(|_| ())
    }

    /// Sets `play_position` if it diverges multiple seconds (10) from the last value.
    /// If it doesn't diverge Ok(()) is returned, nothing is written.
    pub fn set_play_position_if_divergent(&mut self, seconds: i32) -> Result<(), DataError> {
        if seconds != 0 && self.play_position != 0 {
            if (seconds - self.play_position).abs() > 10 {
                return self.set_play_position(seconds);
            }
        } else {
            return self.set_play_position(seconds);
        }
        Ok(())
    }
}

#[derive(Queryable, AsChangeset, PartialEq)]
#[diesel(table_name = episodes)]
#[diesel(treat_none_as_null = true)]
#[diesel(primary_key(title, show_id))]
#[derive(Debug, Clone)]
/// Diesel Model to be used internal with the `utils::checkup` function.
pub struct EpisodeCleanerModel {
    rowid: i32,
    local_uri: Option<String>,
    played: Option<i32>,
}

impl Save<usize> for EpisodeCleanerModel {
    type Error = DataError;

    /// Helper method to easily save/"sync" current state of self to the
    /// Database.
    fn save(&self) -> Result<usize, Self::Error> {
        use crate::schema::episodes::dsl::*;

        let db = connection();
        let mut tempdb = db.get()?;

        diesel::update(episodes.filter(rowid.eq(self.rowid)))
            .set(self)
            .execute(&mut tempdb)
            .map_err(From::from)
    }
}

impl From<Episode> for EpisodeCleanerModel {
    fn from(e: Episode) -> EpisodeCleanerModel {
        EpisodeCleanerModel {
            rowid: e.rowid(),
            local_uri: e.local_uri,
            played: e.played,
        }
    }
}

impl EpisodeCleanerModel {
    /// Get the value of the sqlite's `ROW_ID`
    pub fn rowid(&self) -> i32 {
        self.rowid
    }

    /// Get the value of the `local_uri`.
    ///
    /// Represents the local uri,usually filesystem path,
    /// that the media file will be located at.
    pub fn local_uri(&self) -> Option<&str> {
        self.local_uri.as_deref()
    }

    /// Set the `local_uri`.
    pub fn set_local_uri(&mut self, value: Option<&str>) {
        self.local_uri = value.map(|x| x.to_string());
    }

    /// Epoch representation of the last time the episode was played.
    ///
    /// None/Null for unplayed.
    pub fn played(&self) -> Option<i32> {
        self.played
    }

    /// Set the `played` value.
    pub fn set_played(&mut self, value: Option<i32>) {
        self.played = value;
    }
}

#[derive(Queryable, AsChangeset, PartialEq)]
#[diesel(table_name = episodes)]
#[diesel(treat_none_as_null = true)]
#[diesel(primary_key(title, show_id))]
#[derive(Debug, Clone)]
/// Diesel Model to be used for FIXME.
pub struct EpisodeMinimal {
    rowid: i32,
    title: String,
    uri: Option<String>,
    epoch: i32,
    length: Option<i32>,
    duration: Option<i32>,
    play_position: i32,
    guid: Option<String>,
    show_id: i32,
}

impl From<Episode> for EpisodeMinimal {
    fn from(e: Episode) -> Self {
        EpisodeMinimal {
            rowid: e.rowid,
            title: e.title,
            uri: e.uri,
            length: e.length,
            guid: e.guid,
            epoch: e.epoch,
            duration: e.duration,
            play_position: e.play_position,
            show_id: e.show_id,
        }
    }
}

impl EpisodeMinimal {
    /// Get the value of the sqlite's `ROW_ID`
    pub fn rowid(&self) -> i32 {
        self.rowid
    }

    /// Get the value of the `title` field.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Get the value of the `uri`.
    ///
    /// Represents the url(usually) that the media file will be located at.
    pub fn uri(&self) -> Option<&str> {
        self.uri.as_deref()
    }

    /// Get the Episode's `guid`.
    pub fn guid(&self) -> Option<&str> {
        self.guid.as_deref()
    }

    /// Get the `epoch` value.
    ///
    /// Retrieved from the rss Item publish date.
    /// Value is set to Utc whenever possible.
    pub fn epoch(&self) -> i32 {
        self.epoch
    }

    /// Get the `length`.
    ///
    /// The number represents the size of the file in bytes.
    pub fn length(&self) -> Option<i32> {
        self.length
    }

    /// Get the `duration` value.
    ///
    /// The number represents the duration of the item/episode in seconds.
    pub fn duration(&self) -> Option<i32> {
        self.duration
    }

    /// `Show` table foreign key.
    pub fn show_id(&self) -> i32 {
        self.show_id
    }
}
