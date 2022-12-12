// import { PhotonImage } from 'hypetrigger-wasm'
// import grayscale, { Grayscale } from './_grayscale'
// import identity, { Identity } from './_identity'
// import otsu, { Otsu } from './_otsu'
// import threshold, { Threshold } from './threshold'

// export type Filter = Threshold | Otsu | Grayscale | Identity

// /** A filter transforms one PhotonImage to another PhotonImage */
// export default function filter(image: PhotonImage, filters: Filter[]): PhotonImage {
//   let curImage = image
//   for (const filter of filters)
//     curImage = Filters[filter.type](curImage, filter)
//   return curImage
// }

// export type FilterFunction<F extends Filter> = (image: PhotonImage, filter: F) => PhotonImage

// /**
//  * Connects filter configs to their corresponding function implementation
//  * according to `Filter.type`
//  */
// const Filters: { [key in Filter['type']]: FilterFunction<Filter> } = {
//   threshold,
//   otsu,
//   grayscale,
//   identity,
// }
