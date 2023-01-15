import { Crop, PhotonImage } from '.'

export type TensorflowTrigger = {
  crop?: Crop
}

export const runTensorflowTrigger = (image: PhotonImage, trigger: TensorflowTrigger) => {
  throw new Error('not implemented')
}
