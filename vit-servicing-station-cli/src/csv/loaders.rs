use crate::db_utils::{backup_db_file, restore_db_file};
use crate::{db_utils::db_file_exists, task::ExecTask};
use csv::Trim;
use serde::de::DeserializeOwned;
use std::convert::TryInto;
use std::{fs, io};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use diesel::Insertable;
use structopt::StructOpt;
use thiserror::Error;
use vit_servicing_station_lib::db::models::goals::InsertGoal;
use vit_servicing_station_lib::db::models::proposals::{
    community_choice, simple, ProposalChallengeInfo,
};
use vit_servicing_station_lib::db::{
    load_db_connection_pool,
    models::{funds::Fund, proposals::Proposal, voteplans::Voteplan},
};
use vit_servicing_station_lib::db::models::vote::Vote;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error("Invalid Fund Data: {0}")]
    InvalidFundData(String),
}
#[derive(Debug, Eq, PartialEq, StructOpt)]
pub enum Load {
    /// Load Votes into a SQLite3 ready file DB.
    Votes {
        #[structopt(long = "folder")]
        folder: PathBuf,
    },

    /// Load Funds, Voteplans and Proposals information into a SQLite3 ready file DB.
    All {
        /// Path to the csv containing funds information
        /// At the moment, it's required these are ordered.
        ///
        /// Also the first fund being the current one, which means previous funds should not be
        /// included. This restriction may be lifted in the future.
        #[structopt(long = "funds")]
        funds: PathBuf,

        /// Path to the csv containing voteplans information
        #[structopt(long = "voteplans")]
        voteplans: PathBuf,

        /// Path to the csv containing proposals information
        #[structopt(long = "proposals")]
        proposals: PathBuf,

        /// Path to the csv containing challenges information
        #[structopt(long = "challenges")]
        challenges: PathBuf,

        /// Path to the csv containing advisor reviews information
        #[structopt(long = "reviews")]
        reviews: PathBuf,

        /// Path to the csv containing goals information
        #[structopt(long = "goals")]
        goals: PathBuf,
    },
}

#[derive(Debug, Eq, PartialEq, StructOpt)]
pub struct CsvDataCmd {
    /// URL of the vit-servicing-station database to interact with
    #[structopt(long = "db-url")]
    db_url: String,

    #[structopt(subcommand)]
    command: Load
}

impl CsvDataCmd {
    fn load_from_csv<T: DeserializeOwned>(csv_path: &Path) -> io::Result<Vec<T>> {
        let mut reader = csv::ReaderBuilder::new()
            .flexible(true)
            .has_headers(true)
            .quoting(true)
            .quote(b'"')
            .trim(Trim::All)
            .from_path(csv_path)?;
        let mut results = Vec::new();
        for record in reader.deserialize() {
            match record {
                Ok(data) => {
                    results.push(data);
                }
                Err(e) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            "Error in file {}.\nCause:\n\t{}",
                            csv_path.to_string_lossy(),
                            e
                        ),
                    ))
                }
            }
        }
        Ok(results)
    }

    fn handle_load(
        db_url: &str,
        funds_path: &Path,
        voteplans_path: &Path,
        proposals_path: &Path,
        challenges_path: &Path,
        reviews_path: &Path,
        goals_path: &Path,
    ) -> Result<(), Error> {
        db_file_exists(db_url)?;
        let funds = CsvDataCmd::load_from_csv::<Fund>(funds_path)?;

        let mut voteplans = CsvDataCmd::load_from_csv::<Voteplan>(voteplans_path)?;
        let mut challenges =
            CsvDataCmd::load_from_csv::<super::models::Challenge>(challenges_path)?;
        let csv_proposals = CsvDataCmd::load_from_csv::<super::models::Proposal>(proposals_path)?;
        let reviews = CsvDataCmd::load_from_csv::<super::models::AdvisorReview>(reviews_path)?
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()?;
        let mut goals: Vec<InsertGoal> = CsvDataCmd::load_from_csv::<InsertGoal>(goals_path)?;

        let mut proposals: Vec<Proposal> = Vec::new();
        let mut simple_proposals_data: Vec<simple::ChallengeSqlValues> = Vec::new();
        let mut community_proposals_data: Vec<community_choice::ChallengeSqlValues> = Vec::new();

        for proposal in csv_proposals {
            let challenge_type = challenges
                .iter()
                .find(|c| proposal.challenge_id == c.id)
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Challenge with id {} not found", proposal.challenge_id),
                    )
                })?
                .challenge_type
                .clone();
            let (proposal, challenge_info) =
                proposal.into_db_proposal_and_challenge_info(challenge_type)?;
            match challenge_info {
                ProposalChallengeInfo::Simple(simple) => simple_proposals_data
                    .push(simple.to_sql_values_with_proposal_id(&proposal.proposal_id)),
                ProposalChallengeInfo::CommunityChoice(community_choice) => {
                    community_proposals_data.push(
                        community_choice.to_sql_values_with_proposal_id(&proposal.proposal_id),
                    )
                }
            };
        }

        // start db connection
        let pool = load_db_connection_pool(db_url)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("{}", e)))?;
        let db_conn = pool
            .get()
            .map_err(|e| io::Error::new(io::ErrorKind::NotConnected, format!("{}", e)))?;

        let mut funds_iter = funds.into_iter();

        // insert fund and retrieve fund with id
        let fund = vit_servicing_station_lib::db::queries::funds::insert_fund(
            funds_iter
                .next()
                .ok_or_else(|| Error::InvalidFundData(funds_path.to_string_lossy().to_string()))?,
            &db_conn,
        )
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{}", e)))?;

        for fund in funds_iter {
            vit_servicing_station_lib::db::queries::funds::insert_fund(fund, &db_conn)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{}", e)))?;
        }

        // apply fund id in voteplans
        for voteplan in voteplans.iter_mut() {
            voteplan.fund_id = fund.id;
        }

        // apply fund id in proposals
        for proposal in proposals.iter_mut() {
            proposal.fund_id = fund.id;
        }

        // apply fund id to challenges
        for challenge in challenges.iter_mut() {
            challenge.fund_id = fund.id;
        }

        for goal in goals.iter_mut() {
            goal.fund_id = fund.id;
        }

        vit_servicing_station_lib::db::queries::voteplans::batch_insert_voteplans(
            &voteplans, &db_conn,
        )
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{}", e)))?;

        vit_servicing_station_lib::db::queries::proposals::batch_insert_proposals(
            &proposals, &db_conn,
        )
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{}", e)))?;

        vit_servicing_station_lib::db::queries::proposals::batch_insert_simple_challenge_data(
            &simple_proposals_data,
            &db_conn,
        )
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{}", e)))?;

        vit_servicing_station_lib::db::queries::proposals::batch_insert_community_choice_challenge_data(
            &community_proposals_data,
            &db_conn,
        )
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{}", e)))?;

        vit_servicing_station_lib::db::queries::challenges::batch_insert_challenges(
            &challenges
                .into_iter()
                .map(|c| c.into_db_challenge_values())
                .collect::<Vec<_>>(),
            &db_conn,
        )
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{}", e)))?;

        vit_servicing_station_lib::db::queries::community_advisors_reviews::batch_insert_advisor_reviews(&reviews, &db_conn)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{}", e)))?;

        vit_servicing_station_lib::db::queries::goals::batch_insert(goals, &db_conn)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{}", e)))?;

        Ok(())
    }

    fn list_of_csv_paths<P: AsRef<Path>>(root: P) -> io::Result<Vec<PathBuf>> {
        let mut result = vec![];

        for path in fs::read_dir(root)? {
            let path = path?.path();
            if let Some("csv") = path.extension().and_then(OsStr::to_str) {
                result.push(path.to_owned());
            }
        }
        Ok(result)
    }

    fn handle_votes_load(
        db_url: &str,
        votes_path: &Path,
    ) -> Result<(), Error> {
        db_file_exists(db_url)?;

        let mut votes = vec![];

        for csv_file in Self::list_of_csv_paths(votes_path)? {
            votes.extend(CsvDataCmd::load_from_csv::<Vote>(&csv_file)?);
        }

        // start db connection
        let pool = load_db_connection_pool(db_url)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("{}", e)))?;
        let db_conn = pool
            .get()
            .map_err(|e| io::Error::new(io::ErrorKind::NotConnected, format!("{}", e)))?;


        vit_servicing_station_lib::db::queries::votes::batch_insert_votes_data(
            &votes
                .into_iter()
                .map(|c| c.values())
                .collect::<Vec<_>>(), &db_conn,
        ).map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{}", e)))?;
        Ok(())
    }


    fn handle_load_with_db_backup(
        db_url: &str,
        funds_path: &Path,
        voteplans_path: &Path,
        proposals_path: &Path,
        challenges_path: &Path,
        reviews: &Path,
        goals: &Path,
    ) -> Result<(), Error> {
        let backup_file = backup_db_file(db_url)?;
        if let Err(e) = Self::handle_load(
            db_url,
            funds_path,
            voteplans_path,
            proposals_path,
            challenges_path,
            reviews,
            goals,
        ) {
            restore_db_file(backup_file, db_url)?;
            Err(e)
        } else {
            Ok(())
        }
    }
}

impl ExecTask for CsvDataCmd {
    type ResultValue = ();
    type Error = Error;
    fn exec(&self) -> Result<(), Error> {
        match &self.command {
            Load::All {
                funds,
                voteplans,
                proposals,
                challenges,
                reviews,
                goals,
            } => Self::handle_load_with_db_backup(
                &self.db_url, funds, voteplans, proposals, challenges, reviews, goals,
            ),
            Load::Votes {folder} => {
                let backup_file = backup_db_file(&self.db_url)?;
                if let Err(e) = Self::handle_votes_load(
                    &self.db_url,
                    folder,
                ) {
                    restore_db_file(backup_file, &self.db_url)?;
                    Err(e)
                } else {
                    Ok(())
                }
            }
        }
    }
}