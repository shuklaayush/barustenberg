use std::rc::Rc;

use crate::ecc::fields::field::FieldParams;
use crate::plonk::proof_system::proving_key::ProvingKey;
use crate::transcript::Transcript;

enum WorkType {
    Fft,
    SmallFft,
    Ifft,
    ScalarMultiplication,
}

pub(crate) struct WorkItemInfo {
    num_scalar_multiplications: usize,
    num_ffts: usize,
    num_iffts: usize,
}

struct WorkItem<Fr: FieldParams> {
    work_type: WorkType,
    mul_scalars: Rc<Vec<Fr>>,
    tag: String,
    constant: Fr,
    index: usize,
}

pub(crate) struct QueuedFftInputs<Fr: FieldParams> {
    data: Vec<Fr>,
    shift_factor: Fr,
}

pub(crate) struct WorkQueue<Fr: FieldParams> {
    key: Option<Rc<ProvingKey<Fr>>>,
    transcript: Option<Rc<Transcript<Fr>>>,
    work_items: Vec<WorkItem<Fr>>,
}

impl<Fr: FieldParams> WorkQueue<Fr> {
    /*
    work_item_info get_queued_work_item_info() const;

    barretenberg::fr* get_scalar_multiplication_data(const size_t work_item_number) const;

    size_t get_scalar_multiplication_size(const size_t work_item_number) const;

    barretenberg::fr* get_ifft_data(const size_t work_item_number) const;

    void put_ifft_data(barretenberg::fr* result, const size_t work_item_number);

    queued_fft_inputs get_fft_data(const size_t work_item_number) const;

    void put_fft_data(barretenberg::fr* result, const size_t work_item_number);

    void put_scalar_multiplication_data(const barretenberg::g1::affine_element result, const size_t work_item_number);

    void flush_queue();

    void add_to_queue(const work_item& item);

    void process_queue();

    std::vector<work_item> get_queue() const;
     */

    pub fn new(
        prover_key: Option<Rc<ProvingKey<Fr>>>,
        prover_transcript: Option<Rc<Transcript<Fr>>>,
    ) {
        WorkQueue {
            key: prover_key,
            transcript: prover_transcript,
            work_items: Vec::new(),
        }
    }

    fn get_queued_work_item_info(&self) -> WorkItemInfo {
        let mut num_scalar_multiplications = 0;
        let mut num_ffts = 0;
        let mut num_iffts = 0;
        for item in &self.work_items {
            match item.work_type {
                //WorkType::Fft => num_ffts += 1,
                WorkType::Fft => (),
                WorkType::SmallFft => num_ffts += 1,
                WorkType::Ifft => num_iffts += 1,
                WorkType::ScalarMultiplication => num_scalar_multiplications += 1,
            }
        }
        WorkItemInfo {
            num_scalar_multiplications,
            num_ffts,
            num_iffts,
        }
    }

    fn get_scalar_multiplication_data(&self, work_item_number: usize) -> Option<Rc<Vec<Fr>>> {
        let mut count: usize = 0;
        for item in self.work_items.iter() {
            if item.work_type == WorkType::ScalarMultiplication {
                if count == work_item_number {
                    return Some(item.mul_scalars.clone());
                };
                count += 1;
            }
        }
        None
    }

    fn get_scalar_multiplication_size(&self, work_item_number: usize) -> usize {
        let mut count: usize = 0;
        for item in self.work_items.iter() {
            if item.work_type == WorkType::ScalarMultiplication {
                if count == work_item_number {
                    todo!("look at this nasty code")
                    //return Sstatic_cast<size_t>(static_cast<uint256_t>(item.constant));
                };
                count += 1;
            }
        }
        0
    }

    fn get_ifft_data(&self, work_item_number: usize) -> Option<Rc<Vec<Fr>>> {
        let mut count: usize = 0;
        for item in self.work_items.iter() {
            if item.work_type == WorkType::Ifft {
                if count == work_item_number {
                    todo!("look at this code");
                    // barretenberg::polynomial& wire = key->polynomial_store.get(item.tag + "_lagrange");
                    // return wire.get_coefficients();
                };
                count += 1;
            }
        }
        None
    }

    fn put_ifft_data(&self) {
        todo!("do it");
    }

    fn get_fft_data(&self) {
        todo!("do it");
    }

    fn put_fft_data(&self) {
        todo!("do it")
    }

    fn put_scalar_multiplication_data(&self) {
        todo!("do it")
    }

    pub fn flush_queue(&mut self) {
        self.work_items = vec![];
    }
    fn add_to_queue(&mut self) {
        todo!("whole fucking wasm thing")
    }
    pub fn process_queue(&self) {
        todo!("aaaaagh")
    }
    fn get_queue(&self) {
        todo!("aagh")
    }
}
