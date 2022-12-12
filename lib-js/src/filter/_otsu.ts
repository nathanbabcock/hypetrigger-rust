import { FilterFunction } from '.'

export type Otsu = {
  type: 'otsu'
  rCoef: number
  gCoef: number
  bCoef: number
}

const otsu: FilterFunction<Otsu> = (image, filter) => { throw new Error('Not implemented') }
export default otsu
