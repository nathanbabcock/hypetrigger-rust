import { FilterFunction } from '.'

export type Grayscale = {
  type: 'grayscale'
  rCoef: number
  gCoef: number
  bCoef: number
}

const grayscale: FilterFunction<Grayscale> = (image, filter) => { throw new Error('Not implemented') }
export default grayscale
