import { TesseractTrigger } from './TesseractTrigger'

export type ConfigTrigger =
  | TesseractTrigger & { type: 'ocr' }
  | TesseractTrigger & { type: 'tensorflow' }

export type Config = {
  triggers: ConfigTrigger[]
}

// const configTrigger = (configTrigger: ConfigTrigger) => {
//   switch (configTrigger.type) {
//     case 'ocr': return tesseractTrigger
//     case 'tensorflow': return tensorflowTrigger
//   }
// }