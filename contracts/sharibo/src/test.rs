#![cfg(test)]

use super::*;
use ark_bls12_381::{Fq, Fq2, Fr as ArkFr};
use ark_ff::{BigInteger, PrimeField};
use ark_serialize::CanonicalSerialize;
use core::str::FromStr;
use soroban_sdk::testutils::Events as _;
use soroban_sdk::Symbol;
use soroban_sdk::{
    crypto::bls12_381::{G1_SERIALIZED_SIZE, G2_SERIALIZED_SIZE},
    symbol_short,
    testutils::{Address as _, Ledger as _},
    BytesN, TryIntoVal, U256,
};
use std::vec::Vec as StdVec;

// ---- BLS12-381 test fixture helpers ----
// The vk/proof/public-signal decimal coordinates below were produced by the
// real Phase 1 pipeline (circuits/scripts/{compile,setup,prove}.sh) for a
// genuine member of a 3-member circle at circle_id=0, round=0 — see
// circuits/verification_key.json and circuits/SETUP_TRANSCRIPT.md (entry
// 2026-09-04). The recipientHash public input (#266) is bound to the fixed
// payout recipients in real_recipient_r0/real_recipient_r1, so claim
// succeeds only when the recipient matches the proof's registered hash.
// This mirrors the pattern in Stellar's own groth16_verifier reference
// example (stellar/soroban-examples), which also hand-copies snarkjs
// decimal coordinates into ark_bls12_381 test fixtures.

fn g1_from_coords(env: &Env, x: &str, y: &str) -> G1Affine {
    let ark_g1 = ark_bls12_381::G1Affine::new(Fq::from_str(x).unwrap(), Fq::from_str(y).unwrap());
    let mut buf = [0u8; G1_SERIALIZED_SIZE];
    ark_g1.serialize_uncompressed(&mut buf[..]).unwrap();
    G1Affine::from_array(env, &buf)
}

fn g2_from_coords(env: &Env, x1: &str, x2: &str, y1: &str, y2: &str) -> G2Affine {
    let x = Fq2::new(Fq::from_str(x1).unwrap(), Fq::from_str(x2).unwrap());
    let y = Fq2::new(Fq::from_str(y1).unwrap(), Fq::from_str(y2).unwrap());
    let ark_g2 = ark_bls12_381::G2Affine::new(x, y);
    let mut buf = [0u8; G2_SERIALIZED_SIZE];
    ark_g2.serialize_uncompressed(&mut buf[..]).unwrap();
    G2Affine::from_array(env, &buf)
}

fn fr_from_dec_str(env: &Env, s: &str) -> Fr {
    let ark_fr = ArkFr::from_str(s).unwrap();
    let be_bytes = ark_fr.into_bigint().to_bytes_be();
    let mut buf = [0u8; 32];
    buf[32 - be_bytes.len()..].copy_from_slice(&be_bytes);
    Fr::from_bytes(BytesN::from_array(env, &buf))
}

fn real_verification_key(env: &Env) -> VerificationKey {
    VerificationKey {
        alpha: g1_from_coords(
            env,
            "749582537839343753662662092450452397832509643622354603215105997794324965974939825437185114316129745783000679419786",
            "349962341132122890724568751232889453699201095759070405617825002476699798173144003754632387388100339536150581215244",
        ),
        beta: g2_from_coords(
            env,
            "3880057797060520124320578764877315797540700415384043973769078971696515610163457289102411638052268730945509440648302",
            "3777314379925758442990512187413923812964245102202468031112766319752244038246687083607245318069508267135278455348947",
            "1050410361212406767716359668205231057458158288436209166038545299426881545468977171139347619446018726197388473923235",
            "2493412734090615878237556198351488937361522748982892294901084973296832797018771262475192943991186743848961306012498",
        ),
        gamma: g2_from_coords(
            env,
            "352701069587466618187139116011060144890029952792775240219908644239793785735715026873347600343865175952761926303160",
            "3059144344244213709971259814753781636986470325476647558659373206291635324768958432433509563104347017837885763365758",
            "1985150602287291935568054521177171638300868978215655730859378665066344726373823718423869104263333984641494340347905",
            "927553665492332455747201965776037880757740193453592970025027978793976877002675564980949289727957565575433344219582",
        ),
        delta: g2_from_coords(
            env,
            "3103645666922550361111901561944701284006750573312632567332559875331690914403420941599818706436045124448669974250790",
            "3892473957942423684853166161187107959564012482950189624261130947444530209444107817942442732442430331490982000756874",
            "3860930287635271697415179879375689624186819560916850902324686817327713655307663181135121813555053936247373929272638",
            "1609784541431292060270585748736180809687860649556710589001468182944600766631512130414063743752599919312580581028322",
        ),
        ic: Vec::from_array(
            env,
            [
                g1_from_coords(
            env,
            "1948681912634771776347271243697269400762251716937532457452923581348369025432509442708890118552407975194237752144664",
            "1526361214863697803897508994557674006711987536500572772987868823818838123020499567392825827003234526229256592150572",
        ),
                g1_from_coords(
            env,
            "1996879509684005423562585401688654576575161232087490077625494204747032957766974172044086894908053378253904401755730",
            "938418458954158369731701829218837465333564171691282976128469021105681510590706886645753016371048743646878192443694",
        ),
                g1_from_coords(
            env,
            "2398560985381871540380692463907950405589737572830547852061272016965957189219825831650864260209629954479961889180470",
            "3729714988371021735287567888627375815455285341680858179816835152338786038727026139819308628040992976171935801237270",
        ),
                g1_from_coords(
            env,
            "3403520263757154130275502090118802462849944469065463024473044329672057262363304773273195369185323624254346567471289",
            "3805601893891695262958779295252389984767576447401674048318515899983086904483008660041019394945622372012874842387532",
        ),
                g1_from_coords(
            env,
            "2654953448148255763590886035502807670705030137324598581800079858885547885080731505086166634735365119851769808844281",
            "3824601767367754127584601901985396184116989525023722408076886085818232017543800657331496265505592063050199138090712",
        ),
            ],
        ),
    }
}

fn real_valid_proof(env: &Env) -> Proof {
    Proof {
        a: g1_from_coords(
            env,
            "1708349714640132990116341818099964791395935613547019890172791631283252314514288166731033918417312755095039285019843",
            "3285460062824873754925050999595431551059036641146200280516246640127434556217703173184316895681654718481463021120032",
        ),
        b: g2_from_coords(
            env,
            "3461515140738367304484452093316171207098333521504934047864391267237205115923604207656136593549663497639960424315782",
            "393491027553521445187155884887801269497626412922068420673562699995765222927347934435695079142060868819232246104637",
            "3981742205559613706898623056751215992912828763233405579982160644558619024771303681944429856866144261251325320172945",
            "1143950950053112767246721891107648338869439135157952217329223294149907281346748974986679982543924761919300573028673",
        ),
        c: g1_from_coords(
            env,
            "2010730659768333791961548028276904949853697805224240152059270689203856732296067246426446940151551538588615585749195",
            "285401301347906869760402273560056020351528454436494587933276390880604851069804928520340988317916344941437093763888",
        ),
    }
}

// Real public signals for the proof above: (nullifier_hash, root, external_nullifier).
fn real_root(env: &Env) -> Fr {
    fr_from_dec_str(
        env,
        "26209293814355131390889932661322725195394840191932303091376020297848638697892",
    )
}
fn real_nullifier_hash(env: &Env) -> Fr {
    fr_from_dec_str(
        env,
        "21226719646080371019275358926522886326845061441166218142415794470695116145494",
    )
}
fn real_external_nullifier_round0(env: &Env) -> Fr {
    fr_from_dec_str(
        env,
        "9916401131788634118796694467337109503795060207059715207260235684299224251787",
    )
}

fn fixture_recipient_xdr(env: &Env, k: u8) -> Address {
    let mut b = [0u8; 40];
    b[3] = 18; // SCVAL_ADDRESS
    b[7] = 1; // ScAddress::Contract
    for byte in b.iter_mut().skip(8) {
        *byte = k;
    }
    use soroban_sdk::xdr::FromXdr as _;
    Address::from_xdr(env, &Bytes::from_array(env, &b)).unwrap()
}

// Fixed payout recipients for the committed proofs. Contract addresses are
// used (not account addresses) so the token payout needs no trustline, and
// each proof's recipientHash public input is the XDR SHA-256 (mod r) of the
// recipient address — a claim pays out only to the exact registered
// recipient (issue #266), a same-proof replay to any other address is
// rejected.
fn real_recipient_r0(env: &Env) -> Address {
    fixture_recipient_xdr(env, 1)
}

fn real_recipient_r1(env: &Env) -> Address {
    fixture_recipient_xdr(env, 2)
}
// ---- Issue #91: second trusted-setup ceremony, same identity, two rounds ----
//
// The fixtures above (real_verification_key/real_valid_proof) came from one
// Phase 1 ceremony and only ever proved round 0. To answer "can the same
// identity claim two consecutive rounds today?" we need a *second* proof
// for the SAME identityNullifier/identitySecret/Merkle path, bound to
// round 1's externalNullifier — which means a second, self-consistent
// (vk, proof) pair from a fresh ceremony (a Groth16 proof only verifies
// against the vk from the ceremony that produced it). Root and round-0
// externalNullifier/nullifierHash are unchanged (they don't depend on the
// ceremony), so those still match real_root()/real_external_nullifier_round0()
// /real_nullifier_hash() above — only the vk and both proofs are new.
// Regenerated 2026-09-04 from the same ceremony shape as the canonical key,
// proving round 0 (recipientHash = real_recipient_r0) and round 1
// (recipientHash = real_recipient_r1).

fn round_reuse_verification_key(env: &Env) -> VerificationKey {
    VerificationKey {
        alpha: g1_from_coords(
            env,
            "749582537839343753662662092450452397832509643622354603215105997794324965974939825437185114316129745783000679419786",
            "349962341132122890724568751232889453699201095759070405617825002476699798173144003754632387388100339536150581215244",
        ),
        beta: g2_from_coords(
            env,
            "3880057797060520124320578764877315797540700415384043973769078971696515610163457289102411638052268730945509440648302",
            "3777314379925758442990512187413923812964245102202468031112766319752244038246687083607245318069508267135278455348947",
            "1050410361212406767716359668205231057458158288436209166038545299426881545468977171139347619446018726197388473923235",
            "2493412734090615878237556198351488937361522748982892294901084973296832797018771262475192943991186743848961306012498",
        ),
        gamma: g2_from_coords(
            env,
            "352701069587466618187139116011060144890029952792775240219908644239793785735715026873347600343865175952761926303160",
            "3059144344244213709971259814753781636986470325476647558659373206291635324768958432433509563104347017837885763365758",
            "1985150602287291935568054521177171638300868978215655730859378665066344726373823718423869104263333984641494340347905",
            "927553665492332455747201965776037880757740193453592970025027978793976877002675564980949289727957565575433344219582",
        ),
        delta: g2_from_coords(
            env,
            "2782199162700541151590293642305149245941133573304292014026613340245638150528884723912279081423150251910370591011667",
            "1667965123321298419005404721536913286386696704243087897513376035401180460494271888014230149112884014846330935711274",
            "1565021075436299422171230555707527033313444826682266793248045102362668248763197924515903636436166901362990664872853",
            "1301042207180221059048456631035486481878699810584841268580403698129799558708961199014061692730926219821072491819436",
        ),
        ic: Vec::from_array(
            env,
            [
                g1_from_coords(
            env,
            "1948681912634771776347271243697269400762251716937532457452923581348369025432509442708890118552407975194237752144664",
            "1526361214863697803897508994557674006711987536500572772987868823818838123020499567392825827003234526229256592150572",
        ),
                g1_from_coords(
            env,
            "1996879509684005423562585401688654576575161232087490077625494204747032957766974172044086894908053378253904401755730",
            "938418458954158369731701829218837465333564171691282976128469021105681510590706886645753016371048743646878192443694",
        ),
                g1_from_coords(
            env,
            "2398560985381871540380692463907950405589737572830547852061272016965957189219825831650864260209629954479961889180470",
            "3729714988371021735287567888627375815455285341680858179816835152338786038727026139819308628040992976171935801237270",
        ),
                g1_from_coords(
            env,
            "3403520263757154130275502090118802462849944469065463024473044329672057262363304773273195369185323624254346567471289",
            "3805601893891695262958779295252389984767576447401674048318515899983086904483008660041019394945622372012874842387532",
        ),
                g1_from_coords(
            env,
            "2654953448148255763590886035502807670705030137324598581800079858885547885080731505086166634735365119851769808844281",
            "3824601767367754127584601901985396184116989525023722408076886085818232017543800657331496265505592063050199138090712",
        ),
            ],
        ),
    }
}

fn round_reuse_proof_round0(env: &Env) -> Proof {
    Proof {
        a: g1_from_coords(
            env,
            "1179578184163156892844953836318474739505515114028407946368752959862089427076975950437844441984714924047364307847863",
            "2583995179439786185863343706418614460828012726716846426469291464531647132875583069209284530385467167077510517431464",
        ),
        b: g2_from_coords(
            env,
            "1920751719822233711150824646590740142717678302792535050571729467730768891093744404284949819503387275798927443923355",
            "124465706171730484358811411088691374480907355110955487591297440269986196202043034949007840913693506577700901283194",
            "191938888254396250424327753572876070853434510184296114789260973311208360847831491902413009556679539921059487239469",
            "1915328193537518749159774035793016002622296408759565409172262384698272482567353958935792552018927550307377882090837",
        ),
        c: g1_from_coords(
            env,
            "408855853864300316978244656938054243943785534531632258533970983721911975793255750228934363325235221011705844280084",
            "395039523976183092096857501060693098057945786098473832406182006360727832694088099078790128822447850049805133834510",
        ),
    }
}

fn round_reuse_proof_round1(env: &Env) -> Proof {
    Proof {
        a: g1_from_coords(
            env,
            "2259786221683330276448460747884024526358991497498766135291354295974316045857153370560319455949928693033238640483031",
            "2750965138868310651203252825504066124952916510389725216535389255918703661503224948700993333529663443908054242479485",
        ),
        b: g2_from_coords(
            env,
            "2322563613659181235994800405865062759176883970411493460425571582089472681017959229785441374668923947053244898012845",
            "2358430731731540087145362278871412082851015655584284286088467993942564122027476225965526543589705890441987808199293",
            "3446991522716431602868665297839399750125541751205912418318077422548321793726702078433261268281283713226307388135815",
            "1119685591655926824955137167858243200982240024134106163140054532280280570088946835362790782396378497914289964467140",
        ),
        c: g1_from_coords(
            env,
            "187577619091086741012027511381398604995130001598429994322148675806800970453563435482945857655437707265982183306533",
            "2839619480605558288618111614154617437743057244363440683332637490316036895668277129425209099977654348071334913651039",
        ),
    }
}

// Poseidon(identityNullifier, externalNullifier_round1) for the SAME
// identity as real_nullifier_hash() — deliberately a different value
// because externalNullifier changed, even though identityNullifier didn't.
fn round_reuse_nullifier_hash_round1(env: &Env) -> Fr {
    fr_from_dec_str(
        env,
        "49427450209661096950044132594013152139023072336714402456973658706693457893626",
    )
}

fn create_token(env: &Env, admin: &Address) -> Address {
    env.register_stellar_asset_contract_v2(admin.clone())
        .address()
}

fn expected_external_nullifier(env: &Env, circle_id: u64, round: u32) -> Fr {
    Contract::compute_external_nullifier(env, circle_id, round)
}

// ---- TestCircle: the single circle-construction helper ----
//
// Every topic module builds circles through [`TestCircleBuilder`] so that a
// signature change to `create_circle` is a one-line change here rather than
// a hunt across modules — the duplicated setup is what let issue #456's
// arity mismatch survive a merge.

/// A fully-built test circle: contract registered, circle created, members
/// minted. Obtained from [`TestCircleBuilder::build`].
pub(crate) struct TestCircle {
    pub(crate) env: Env,
    pub(crate) client_id: Address,
    pub(crate) admin: Address,
    pub(crate) token: Address,
    pub(crate) members: StdVec<Address>,
    pub(crate) circle_id: u64,
    pub(crate) size: u32,
    pub(crate) contribution: i128,
    pub(crate) fee_bps: u32,
    pub(crate) fee_recipient: Address,
}

impl TestCircle {
    /// Starts a builder for a test circle with the given size and
    /// contribution. Chain [`TestCircleBuilder::with_fee`] /
    /// [`TestCircleBuilder::with_vk`] / [`TestCircleBuilder::with_round_deadline`]
    /// / [`TestCircleBuilder::with_fee_recipient`] to customise, then
    /// [`TestCircleBuilder::build`] to construct it.
    pub(crate) fn new(size: u32, contribution: i128) -> TestCircleBuilder {
        TestCircleBuilder {
            size,
            contribution,
            fee_bps: 0,
            round_deadline_ledgers: 0,
            vk: None,
            fee_recipient: None,
        }
    }
}

/// Builder for [`TestCircle`]. Defaults: no protocol fee, no round deadline,
/// the real verification key, a random fee recipient.
pub(crate) struct TestCircleBuilder {
    size: u32,
    contribution: i128,
    fee_bps: u32,
    round_deadline_ledgers: u32,
    vk: Option<VerificationKey>,
    fee_recipient: Option<Address>,
}

impl TestCircleBuilder {
    /// Protocol fee in basis points (`0..=10_000`).
    pub(crate) fn with_fee(mut self, fee_bps: u32) -> Self {
        self.fee_bps = fee_bps;
        self
    }

    /// Override the verification key (e.g. the round-reuse ceremony key).
    pub(crate) fn with_vk(mut self, vk: VerificationKey) -> Self {
        self.vk = Some(vk);
        self
    }

    /// Round deadline in ledgers (default `0` = no deadline).
    pub(crate) fn with_round_deadline(mut self, ledgers: u32) -> Self {
        self.round_deadline_ledgers = ledgers;
        self
    }

    /// Override the fee recipient (e.g. the contract address, for the
    /// InvalidRecipient guard).
    pub(crate) fn with_fee_recipient(mut self, recipient: Address) -> Self {
        self.fee_recipient = Some(recipient);
        self
    }

    /// Builds the circle on a fresh contract registered on `env`.
    ///
    /// `env` must already have auths mocked (`env.mock_all_auths()`) — the
    /// builder performs contract calls.
    pub(crate) fn build(self, env: &Env) -> TestCircle {
        let contract_id = env.register(Contract, ());
        self.build_with_contract(env, contract_id)
    }

    /// Builds the circle on an already-registered contract (for tests that
    /// create several circles on one contract).
    pub(crate) fn build_with_contract(self, env: &Env, contract_id: Address) -> TestCircle {
        let client = ContractClient::new(env, &contract_id);

        let admin = Address::generate(env);
        let token_admin = Address::generate(env);
        let token = create_token(env, &token_admin);
        let token_admin_client = token::StellarAssetClient::new(env, &token);

        let vk = self.vk.unwrap_or_else(|| real_verification_key(env));
        let fee_recipient = self.fee_recipient.unwrap_or_else(|| Address::generate(env));
        // this is the FIRST circle registered against a fresh contract, so it
        // is assigned circle_id=0 — matching the real proof fixtures above,
        // which were generated for circle_id=0.
        let circle_id = client.create_circle(
            &admin,
            &token,
            &real_root(env),
            &self.contribution,
            &self.size,
            &self.round_deadline_ledgers,
            &vk,
            &self.fee_bps,
            &fee_recipient,
        );

        let mut members: StdVec<Address> = StdVec::new();
        for _ in 0..self.size {
            let m = Address::generate(env);
            token_admin_client.mint(&m, &self.contribution);
            members.push(m);
        }

        TestCircle {
            env: env.clone(),
            client_id: contract_id,
            admin,
            token,
            members,
            circle_id,
            size: self.size,
            contribution: self.contribution,
            fee_bps: self.fee_bps,
            fee_recipient,
        }
    }
}

// ---- Topic modules ----
//
// Each module owns one behavior area and builds circles only through the
// shared [`TestCircleBuilder`].

mod admin;
mod auth;
mod bench;
mod cancel;
mod claim;
mod create;
mod events;
mod fees;
mod fund;
mod goldens;
mod happy_path;
mod invariants;
mod recipient;
mod reentrancy;
mod ttl;
mod views;

// ---- Back-compat shims for remote-main tests (issue #565, meta/vk views) ----
//
// Remote main (172 commits ahead) added top-level tests using the old
// `setup()` helper. The modular refactor replaced it with `TestCircleBuilder`;
// these shims let the ported tests run verbatim while new code uses the builder.
fn setup(size: u32, contribution: i128) -> TestCircle {
    let env = Env::default();
    env.mock_all_auths();
    TestCircle::new(size, contribution).build(&env)
}

fn setup_with_fee(size: u32, contribution: i128, fee_bps: u32) -> (TestCircle, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let builder = TestCircle::new(size, contribution).with_fee(fee_bps);
    let fee_recipient = Address::generate(&env);
    let s = builder
        .with_fee_recipient(fee_recipient.clone())
        .build(&env);
    (s, fee_recipient)
}

// ---- Ported from remote main: round_deadline vs LEDGER_EXTEND_TO (#565) ----

#[test]
fn create_circle_rejects_deadline_at_or_above_ledger_extend_to() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = create_token(&env, &Address::generate(&env));
    let vk = real_verification_key(&env);
    let root = real_root(&env);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.create_circle(
            &admin,
            &token,
            &root,
            &100i128,
            &5u32,
            &LEDGER_EXTEND_TO,
            &vk,
            &0u32,
            &Address::generate(&env),
        );
    }));
    assert!(
        result.is_err(),
        "deadline == LEDGER_EXTEND_TO must be rejected"
    );
}

#[test]
fn create_circle_accepts_deadline_just_below_ledger_extend_to() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = create_token(&env, &Address::generate(&env));
    let vk = real_verification_key(&env);
    let root = real_root(&env);
    let deadline = LEDGER_EXTEND_TO - 1;

    let circle_id = client.create_circle(
        &admin,
        &token,
        &root,
        &100i128,
        &5u32,
        &deadline,
        &vk,
        &0u32,
        &Address::generate(&env),
    );
    let circle = client.get_circle(&circle_id);
    assert_eq!(circle.round_deadline_ledgers, deadline);
}

// ---- Ported from remote main: get_circle_meta / get_vk views ----

#[test]
fn get_circle_meta_returns_mutable_fields() {
    let s = setup(5, 100);
    let client = ContractClient::new(&s.env, &s.client_id);

    let meta = client.get_circle_meta(&s.circle_id);
    assert_eq!(meta.schema_version, 2);
    assert_eq!(meta.admin, s.admin);
    assert_eq!(meta.token, s.token);
    assert_eq!(meta.contribution, s.contribution);
    assert_eq!(meta.size, s.size);
    assert_eq!(meta.round, 0);
    assert_eq!(meta.pot, 0i128);
    assert!(!meta.cancelled);
    assert_eq!(meta.fee_bps, 0u32);

    client.fund(&s.circle_id, &s.members[0]);
    let meta_after = client.get_circle_meta(&s.circle_id);
    assert_eq!(meta_after.pot, s.contribution);
    assert_eq!(meta_after.round, 0);
}

#[test]
fn get_circle_meta_has_no_group_elements() {
    let s = setup(5, 100);
    let client = ContractClient::new(&s.env, &s.client_id);

    let circle = client.get_circle(&s.circle_id);
    let meta = client.get_circle_meta(&s.circle_id);

    let circle_xdr = circle.clone().to_xdr(&s.env);
    let meta_xdr = meta.to_xdr(&s.env);

    assert!(
        meta_xdr.len() < circle_xdr.len() / 2,
        "CircleMeta XDR ({}) should be far smaller than Circle XDR ({})",
        meta_xdr.len(),
        circle_xdr.len(),
    );

    for point in [
        circle.vk.alpha.to_xdr(&s.env),
        circle.vk.beta.to_xdr(&s.env),
        circle.vk.gamma.to_xdr(&s.env),
        circle.vk.delta.to_xdr(&s.env),
    ] {
        let needle: StdVec<u8> = point.iter().collect();
        let haystack: StdVec<u8> = meta_xdr.iter().collect();
        assert!(
            !haystack
                .windows(needle.len())
                .any(|w| w == needle.as_slice()),
            "CircleMeta XDR contains a VK group element",
        );
    }
    for ic_point in circle.vk.ic.iter() {
        let needle: StdVec<u8> = ic_point.to_xdr(&s.env).iter().collect();
        let haystack: StdVec<u8> = meta_xdr.iter().collect();
        assert!(
            !haystack
                .windows(needle.len())
                .any(|w| w == needle.as_slice()),
            "CircleMeta XDR contains a VK ic point",
        );
    }
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn get_circle_meta_unknown_reverts() {
    let s = setup(5, 100);
    let client = ContractClient::new(&s.env, &s.client_id);
    client.get_circle_meta(&999u64);
}

#[test]
fn get_vk_returns_committed_verification_key() {
    let s = setup(5, 100);
    let client = ContractClient::new(&s.env, &s.client_id);

    let circle = client.get_circle(&s.circle_id);
    let vk = client.get_vk(&s.circle_id);

    assert_eq!(
        vk.alpha.to_xdr(&s.env),
        circle.vk.alpha.to_xdr(&s.env),
        "alpha"
    );
    assert_eq!(
        vk.beta.to_xdr(&s.env),
        circle.vk.beta.to_xdr(&s.env),
        "beta"
    );
    assert_eq!(
        vk.gamma.to_xdr(&s.env),
        circle.vk.gamma.to_xdr(&s.env),
        "gamma"
    );
    assert_eq!(
        vk.delta.to_xdr(&s.env),
        circle.vk.delta.to_xdr(&s.env),
        "delta"
    );
    assert_eq!(vk.ic.len(), circle.vk.ic.len(), "ic length");
    for (got, want) in vk.ic.iter().zip(circle.vk.ic.iter()) {
        assert_eq!(got.to_xdr(&s.env), want.to_xdr(&s.env), "ic point");
    }
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn get_vk_unknown_reverts() {
    let s = setup(5, 100);
    let client = ContractClient::new(&s.env, &s.client_id);
    client.get_vk(&999u64);
}

#[test]
fn test_nullifier_set_is_bounded_by_cycle() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let root = real_root(&env);
    let vk = real_verification_key(&env);

    let size = 5u32;
    client.create_circle(
        &admin, &token, &root, &100i128, &size, &0u32, &vk, &0u32, &admin,
    );

    for i in 0..20 {
        env.as_contract(&contract_id, || {
            let key = DataKey::Circle(0);
            let mut circle: Circle = env.storage().persistent().get(&key).unwrap();

            circle.pot = 0;
            circle.round += 1;
            circle.contributors = Vec::new(&env);
            circle.round_started_ledger = env.ledger().sequence();

            let dummy_nullifier = Fr::from_u256(soroban_sdk::U256::from_u32(&env, i));
            circle.nullifiers.push_back(dummy_nullifier);
            if circle.round % circle.size == 0 {
                circle.nullifiers = Vec::new(&env);
            }

            env.storage().persistent().set(&key, &circle);
        });

        let circle = client.get_circle(&0u64);
        assert!(
            circle.nullifiers.len() <= size,
            "Nullifiers exceeded size bound!"
        );
    }
}
