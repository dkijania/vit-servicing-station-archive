table! {
    api_tokens (token) {
        token -> Binary,
        creation_time -> BigInt,
        expire_time -> BigInt,
    }
}

table! {
    challenges (id, challenge_url) {
        internal_id -> Nullable<Integer>,
        id -> Integer,
        challenge_type -> Text,
        title -> Text,
        description -> Text,
        rewards_total -> BigInt,
        proposers_rewards -> BigInt,
        fund_id -> Integer,
        challenge_url -> Text,
        highlights -> Nullable<Text>,
    }
}

table! {
    community_advisors_reviews (id, proposal_id) {
        id -> Integer,
        proposal_id -> Integer,
        assessor -> Text,
        impact_alignment_rating_given -> Nullable<Integer>,
        impact_alignment_note -> Nullable<Text>,
        feasibility_rating_given -> Nullable<Integer>,
        feasibility_note -> Nullable<Text>,
        auditability_rating_given -> Nullable<Integer>,
        auditability_note -> Nullable<Text>,
        ranking -> Nullable<Integer>,
        rating_given -> Nullable<Integer>,
        tag -> Nullable<Text>,
        note -> Nullable<Text>,
    }
}

table! {
    funds (id) {
        id -> Integer,
        fund_name -> Text,
        fund_goal -> Text,
        registration_snapshot_time -> BigInt,
        next_registration_snapshot_time -> BigInt,
        voting_power_threshold -> BigInt,
        fund_start_time -> BigInt,
        fund_end_time -> BigInt,
        next_fund_start_time -> BigInt,
        insight_sharing_start -> BigInt,
        proposal_submission_start -> BigInt,
        refine_proposals_start -> BigInt,
        finalize_proposals_start -> BigInt,
        proposal_assessment_start -> BigInt,
        assessment_qa_start -> BigInt,
        snapshot_start -> BigInt,
        voting_start -> BigInt,
        voting_end -> BigInt,
        tallying_end -> BigInt,
        results_url -> Nullable<Text>,
        survey_url -> Nullable<Text>,
    }
}

table! {
    goals (id, fund_id) {
        id -> Integer,
        goal_name -> Text,
        fund_id -> Integer,
    }
}

table! {
    proposal_community_choice_challenge (proposal_id) {
        proposal_id -> Text,
        proposal_brief -> Nullable<Text>,
        proposal_importance -> Nullable<Text>,
        proposal_goal -> Nullable<Text>,
        proposal_metrics -> Nullable<Text>,
    }
}

table! {
    proposal_simple_challenge (proposal_id) {
        proposal_id -> Text,
        proposal_solution -> Nullable<Text>,
    }
}

table! {
    proposals (id, proposal_id) {
        id -> Integer,
        proposal_id -> Text,
        proposal_category -> Text,
        proposal_title -> Text,
        proposal_summary -> Text,
        proposal_public_key -> Text,
        proposal_funds -> BigInt,
        proposal_url -> Text,
        proposal_files_url -> Text,
        proposal_impact_score -> BigInt,
        proposer_name -> Text,
        proposer_contact -> Text,
        proposer_url -> Text,
        proposer_relevant_experience -> Text,
        chain_proposal_id -> Binary,
        chain_proposal_index -> BigInt,
        chain_vote_options -> Text,
        chain_voteplan_id -> Text,
        challenge_id -> Integer,
    }
}

table! {
    voteplans (chain_voteplan_id) {
        id -> Integer,
        chain_voteplan_id -> Text,
        chain_vote_start_time -> BigInt,
        chain_vote_end_time -> BigInt,
        chain_committee_end_time -> BigInt,
        chain_voteplan_payload -> Text,
        chain_vote_encryption_key -> Text,
        fund_id -> Integer,
    }
}

table! {
    votes (fragment_id) {
        fragment_id -> Text,
        caster -> Text,
        proposal -> Integer,
        voteplan_id -> Text,
        time -> Float,
        choice -> Nullable<SmallInt>,
        raw_fragment -> Text,
    }
}

allow_tables_to_appear_in_same_query!(
    api_tokens,
    challenges,
    community_advisors_reviews,
    funds,
    goals,
    proposal_community_choice_challenge,
    proposal_simple_challenge,
    proposals,
    voteplans,
    votes,
);
