use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
    sync::Arc,
};

use crate::{
    ecc::fields::field::{Field, FieldParams},
    plonk::proof_system::{
        proving_key::ProvingKey,
        types::{
            polynomial_manifest::{EvaluationType, PolynomialIndex},
            prover_settings::Settings,
            PolynomialManifest,
        },
    },
    transcript::{BarretenHasher, Transcript, TranscriptKey},
};

use self::containers::{ChallengeArray, CoefficientArray, PolyArray, PolyPtrMap};

use super::arithmetic_widget::Getters;

pub enum ChallengeIndex {
    Alpha,
    Beta,
    Gamma,
    Eta,
    Zeta,
    MaxNumChallenges,
}

pub const CHALLENGE_BIT_ALPHA: usize = 1 << (ChallengeIndex::Alpha as usize);
pub const CHALLENGE_BIT_BETA: usize = 1 << (ChallengeIndex::Beta as usize);
pub const CHALLENGE_BIT_GAMMA: usize = 1 << (ChallengeIndex::Gamma as usize);
pub const CHALLENGE_BIT_ETA: usize = 1 << (ChallengeIndex::Eta as usize);
pub const CHALLENGE_BIT_ZETA: usize = 1 << (ChallengeIndex::Zeta as usize);

pub mod containers {
    use crate::plonk::proof_system::types::polynomial_manifest::PolynomialIndex;

    use super::ChallengeIndex;
    use std::collections::HashMap;

    pub struct ChallengeArray<Field, const NUM_WIDGET_RELATIONS: usize> {
        pub elements: [Field; ChallengeIndex::MaxNumChallenges as usize],
        pub alpha_powers: [Field; NUM_WIDGET_RELATIONS],
    }

    pub type PolyArray<Field> = [(Field, Field); PolynomialIndex::MaxNumPolynomials as usize];

    pub struct PolyPtrMap<Field> {
        pub coefficients: HashMap<PolynomialIndex, Vec<Field>>,
        pub block_mask: usize,
        pub index_shift: usize,
    }

    pub type CoefficientArray<Field> = [Field; PolynomialIndex::MaxNumPolynomials as usize];
}

// Getters are various structs that are used to retrieve/query various objects needed during the proof.
//
// You can query:
// - Challenges
// - Polynomial evaluations
// - Polynomials in monomial form
// - Polynomials in Lagrange form

/// Implements loading challenges from the transcript and computing powers of α, which are later used in widgets.
///
/// # Type Parameters
/// - `Field`: Base field
/// - `Transcript`: Transcript struct
/// - `Settings`: Configuration
/// - `NUM_WIDGET_RELATIONS`: How many powers of α are needed
pub trait BaseGetter<
    H: BarretenHasher,
    F: FieldParams,
    S: Settings<H>,
    const NUM_WIDGET_RELATIONS: usize,
>
{
    /// Create a challenge array from transcript.
    /// Loads alpha, beta, gamma, eta, zeta, and nu and calculates powers of alpha.
    ///
    /// # Arguments
    /// - `transcript`: Transcript to get challenges from
    /// - `alpha_base`: α to some power (depends on previously used widgets)
    /// - `required_challenges`: Challenge bitmask, which shows when the function should fail
    ///
    /// # Returns
    /// A structure with an array of challenge values and powers of α
    fn get_challenges(
        transcript: &Transcript<H>,
        alpha_base: F,
        required_challenges: u8,
    ) -> ChallengeArray<F, NUM_WIDGET_RELATIONS> {
        let mut result = ChallengeArray::default();
        let add_challenge = |label: &str, tag: usize, required: bool, index: usize| {
            assert!(!required || transcript.has_challenge(label));
            if transcript.has_challenge(label) {
                assert!(index < transcript.get_num_challenges(label));
                result.elements[tag] = transcript.get_challenge_field_element(label, index);
            } else {
                result.elements[tag] = Field::random_element();
            }
        };
        add_challenge(
            "alpha",
            ChallengeIndex::Alpha as usize,
            required_challenges & CHALLENGE_BIT_ALPHA != 0,
            0,
        );
        add_challenge(
            "beta",
            ChallengeIndex::Beta as usize,
            required_challenges & CHALLENGE_BIT_BETA != 0,
            0,
        );
        add_challenge(
            "beta",
            ChallengeIndex::Gamma as usize,
            required_challenges & CHALLENGE_BIT_GAMMA != 0,
            1,
        );
        add_challenge(
            "eta",
            ChallengeIndex::Eta as usize,
            required_challenges & CHALLENGE_BIT_ETA != 0,
            0,
        );
        add_challenge(
            "z",
            ChallengeIndex::Zeta as usize,
            required_challenges & CHALLENGE_BIT_ZETA != 0,
            0,
        );
        result.alpha_powers[0] = alpha_base;
        for i in 1..NUM_WIDGET_RELATIONS {
            result.alpha_powers[i] =
                result.alpha_powers[i - 1] * result.elements[ChallengeIndex::Alpha as usize];
        }
        result
    }

    fn update_alpha(
        challenges: &ChallengeArray<F, NUM_WIDGET_RELATIONS>,
        num_independent_relations: usize,
    ) -> F {
        if num_independent_relations == 0 {
            challenges.alpha_powers[0]
        } else {
            challenges.alpha_powers[num_independent_relations - 1]
                * challenges.elements[ChallengeIndex::Alpha as usize]
        }
    }
}

/// Implements loading polynomial openings from transcript in addition to BaseGetter's
/// loading challenges from the transcript and computing powers of α
pub trait EvaluationGetter<F, H: BarretenHasher, S: Settings<H>, const NUM_WIDGET_RELATIONS: usize>:
    BaseGetter<H, F, S, NUM_WIDGET_RELATIONS>
where
    F: FieldParams,
{
    /// Get a polynomial at offset `id`
    ///
    /// # Arguments
    ///
    /// * `polynomials` - An array of polynomials
    ///
    /// # Type Parameters
    ///
    /// * `use_shifted_evaluation` - Whether to pick first or second
    /// * `id` - Polynomial index.
    ///
    /// # Returns
    ///
    /// The chosen polynomial
    fn get_value<const USE_SHIFTED_EVALUATION: bool, const ID: usize>(
        polynomials: &PolyArray<F>,
    ) -> &F {
        if USE_SHIFTED_EVALUATION {
            &polynomials[ID].1
        } else {
            &polynomials[ID].0
        }
    }

    /// Return an array with poly
    ///
    /// # Arguments
    ///
    /// * `polynomial_manifest`
    /// * `transcript`
    ///
    /// # Returns
    ///
    /// `PolyArray`
    fn get_polynomial_evaluations(
        polynomial_manifest: &PolynomialManifest,
        transcript: &Transcript<H>,
    ) -> PolyArray<F> {
        let mut result: PolyArray<Field> = Default::default();
        for i in 0..polynomial_manifest.len() {
            let info = &polynomial_manifest[i];
            let label = info.polynomial_label.clone();
            result[info.index].0 = transcript.get_field_element(&label);

            if info.requires_shifted_evaluation {
                result[info.index].1 = transcript.get_field_element(&(label + "_omega"));
            } else {
                result[info.index].1 = Field::zero();
            }
        }
        result
    }
}

/// Provides access to polynomials (monomial or coset FFT) for use in widgets
/// Coset FFT access is needed in quotient construction.
pub trait FFTGetter<F, Transcript, Settings, const NUM_WIDGET_RELATIONS: usize>:
    BaseGetter<F, Transcript, Settings, NUM_WIDGET_RELATIONS>
where
    F: FieldParams,
{
    fn get_polynomials(
        key: &ProvingKey<F>,
        required_polynomial_ids: &HashSet<PolynomialIndex>,
    ) -> PolyPtrMap<F> {
        let mut result = PolyPtrMap::new();
        let label_suffix = "_fft";

        result.block_mask = key.large_domain.size - 1;
        result.index_shift = 4;

        for info in &key.polynomial_manifest {
            if required_polynomial_ids.contains(&info.index) {
                let label = info.polynomial_label.clone() + label_suffix;
                result
                    .coefficients
                    .insert(info.index, key.polynomial_store.get(&label));
            }
        }
        result
    }

    fn get_value<const EVALUATION_TYPE: EvaluationType, const ID: usize>(
        polynomials: &PolyPtrMap<F>,
        index: usize,
    ) -> &F {
        if EVALUATION_TYPE == EvaluationType::Shifted {
            let shifted_index = (index + polynomials.index_shift) & polynomials.block_mask;
            &polynomials.coefficients[&ID][shifted_index]
        } else {
            &polynomials.coefficients[&ID][index]
        }
    }
}

pub struct TransitionWidgetBase<F: FieldParams> {
    pub key: Option<Arc<ProvingKey<F>>>,
}

impl<F: FieldParams> TransitionWidgetBase<F> {
    pub fn new(key: Option<Arc<ProvingKey<F>>>) -> Self {
        Self { key }
    }

    // other methods and trait implementations
}

trait KernelBaseTrait {}

trait KernelBase<F: FieldParams, PC, G: Getters<F, PC>, const NUM_INDEPENDENT_RELATIONS: usize>:
    KernelBaseTrait
{
    fn get_required_polynomial_ids() -> HashSet<PolynomialIndex>;
    fn quotient_required_challenges() -> u8;
    fn update_required_challenges() -> u8;
    fn compute_linear_terms(
        polynomials: &PolyPtrMap<F>,
        challenges: &ChallengeArray<F, NUM_INDEPENDENT_RELATIONS>,
        linear_terms: &mut CoefficientArray<F>,
        index: usize,
    );
    fn sum_linear_terms(
        polynomials: &PolyPtrMap<F>,
        challenges: &ChallengeArray<F, NUM_INDEPENDENT_RELATIONS>,
        linear_terms: &CoefficientArray<F>,
        index: usize,
    ) -> F;
    fn compute_non_linear_terms(
        polynomials: &PolyPtrMap<F>,
        challenges: &ChallengeArray<F, NUM_INDEPENDENT_RELATIONS>,
        quotient_term: &mut F,
        index: usize,
    );
}

pub struct TransitionWidget<
    H: BarretenHasher,
    F: FieldParams,
    S: Settings<H>,
    PC,
    G: Getters<F, PC>,
    const NUM_INDEPENDENT_RELATIONS: usize,
    KB: KernelBaseTrait,
> {
    base: TransitionWidgetBase<F>,
    phantom: std::marker::PhantomData<S>,
}

impl<
        H: BarretenHasher,
        F: FieldParams,
        S: Settings<H>,
        PC,
        G: Getters<F, PC>,
        const NUM_INDEPENDENT_RELATIONS: usize,
        KB: KernelBaseTrait,
    > TransitionWidget<H, F, S, PC, G, { NUM_INDEPENDENT_RELATIONS }, KB>
{
    pub fn new(key: Option<Arc<ProvingKey<F>>>) -> Self {
        Self {
            base: TransitionWidgetBase::new(key),
            phantom: std::marker::PhantomData,
        }
    }

    // other methods and trait implementations
    pub fn compute_quotient_contribution(&self, alpha_base: F, transcript: &Transcript<H>) -> F {
        let key = self.base.key.as_ref().expect("Proving key is missing");

        let required_polynomial_ids = KernelBase::get_required_polynomial_ids();
        let polynomials =
            FFTGetter::<F, _, _, KernelBase::NUM_INDEPENDENT_RELATIONS>::get_polynomials(
                key,
                &required_polynomial_ids,
            );

        let challenges =
            FFTGetter::<F, _, _, KernelBase::NUM_INDEPENDENT_RELATIONS>::get_challenges(
                transcript,
                &alpha_base,
                KernelBase::quotient_required_challenges(),
            );

        let mut quotient_term;

        for i in key.large_domain.iter() {
            let mut linear_terms = CoefficientArray::default();
            KernelBase::compute_linear_terms(&polynomials, &challenges, &mut linear_terms, i);
            let sum_of_linear_terms =
                KernelBase::sum_linear_terms(&polynomials, &challenges, &linear_terms, i);

            quotient_term = key.get_quotient_polynomial_part_mut(i);
            *quotient_term += sum_of_linear_terms;
            KernelBase::compute_non_linear_terms(&polynomials, &challenges, quotient_term, i);
        }

        FFTGetter::<Field, _, _, KernelBase::NUM_INDEPENDENT_RELATIONS>::update_alpha(
            &challenges,
            KernelBase::NUM_INDEPENDENT_RELATIONS,
        )
    }
}

// Implementations for the derived classes
impl<F, S, H, KernelBase, PC, G, const NUM_INDEPENDENT_RELATIONS: usize>
    From<TransitionWidget<H, F, S, PC, G, { NUM_INDEPENDENT_RELATIONS }, KernelBase>>
    for TransitionWidgetBase<F>
{
    // Other methods and trait implementations
}

pub struct GenericVerifierWidget<F, Hash, Settings, KernelBase>
where
    F: FieldParams,
{
    phantom: PhantomData<(F, Hash, Settings)>,
}

impl<H, F, Settings, KernelBase> GenericVerifierWidget<F, H, Settings, KernelBase>
where
    F: FieldParams,
{
    pub fn compute_quotient_evaluation_contribution(
        key: &Arc<TranscriptKey>,
        alpha_base: F,
        transcript: &Transcript<H>,
        quotient_numerator_eval: &mut F,
    ) -> F {
        let polynomial_evaluations = EvaluationGetter::<
            F,
            _,
            _,
            KernelBase::NUM_INDEPENDENT_RELATIONS,
        >::get_polynomial_evaluations(
            &key.polynomial_manifest, transcript
        );
        let challenges =
            EvaluationGetter::<Field, _, _, KernelBase::NUM_INDEPENDENT_RELATIONS>::get_challenges(
                transcript,
                &alpha_base,
                KernelBase::quotient_required_challenges(),
            );

        let mut linear_terms = CoefficientArray::default();
        KernelBase::compute_linear_terms(&polynomial_evaluations, &challenges, &mut linear_terms);
        *quotient_numerator_eval +=
            KernelBase::sum_linear_terms(&polynomial_evaluations, &challenges, &linear_terms);
        KernelBase::compute_non_linear_terms(
            &polynomial_evaluations,
            &challenges,
            quotient_numerator_eval,
        );

        EvaluationGetter::<Field, _, _, KernelBase::NUM_INDEPENDENT_RELATIONS>::update_alpha(
            &challenges,
            KernelBase::NUM_INDEPENDENT_RELATIONS,
        )
    }

    pub fn append_scalar_multiplication_inputs(
        key: &Arc<TranscriptKey>,
        alpha_base: F,
        transcript: &Transcript<H>,
        scalar_mult_inputs: &mut HashMap<String, F>,
    ) -> F {
        let challenges =
            EvaluationGetter::<F, _, _, KernelBase::NUM_INDEPENDENT_RELATIONS>::get_challenges(
                transcript,
                &alpha_base,
                KernelBase::quotient_required_challenges()
                    | KernelBase::update_required_challenges(),
            );

        EvaluationGetter::<F, _, _, KernelBase::NUM_INDEPENDENT_RELATIONS>::update_alpha(
            &challenges,
            KernelBase::NUM_INDEPENDENT_RELATIONS,
        )
    }
}
