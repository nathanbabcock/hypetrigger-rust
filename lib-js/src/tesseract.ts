import { Crop, ensure_minimum_size, padding_uniform, PhotonImage, Rgba, ThresholdFilter, putImageData } from '..'
import { createScheduler, createWorker, RecognizeResult, Scheduler, Worker } from 'tesseract.js'
import Tesseract from 'tesseract.js'
import { Trigger } from './trigger'

export class TesseractTrigger extends Trigger {
  public crop?: Crop
  public threshold?: ThresholdFilter
  public scheduler: Scheduler
  public onText?: (text: string) => void
  private internalCanvas: HTMLCanvasElement = document.createElement('canvas')

  preprocess(image: PhotonImage): PhotonImage {
    // if (this.crop) image = this.crop.apply(image)
    // ensure_minimum_size(image, 32)
    // if (this.threshold) image = this.threshold.apply(image)
    // image = padding_uniform(image, 32, new Rgba(255, 255, 255, 255))
    return image
  }

  async recognizeText(image: PhotonImage): Promise<string> {
    this.internalCanvas.width = image.get_width()
    this.internalCanvas.height = image.get_height()
    putImageData(this.internalCanvas, this.internalCanvas.getContext('2d'), image)
    const result = await this.scheduler.addJob('recognize', this.internalCanvas) as RecognizeResult
    const text = result.data.text
    return text
  }

  async run(image: PhotonImage) {
    image = this.preprocess(image)
    let text = await this.recognizeText(image)
    this.onText?.(text)
  }

  constructor(scheduler: Scheduler) {
    super()
    this.scheduler = scheduler
  }
}

export type TesseractOptions = {
  numWorkers: number,
  langs: string
  workerOptions?: Partial<Tesseract.WorkerOptions>,
  workerParams?: Partial<Tesseract.WorkerParams>,
}

export const TesseractDefaults: TesseractOptions = {
  numWorkers: 3,
  langs: 'eng',
  workerOptions: {
    errorHandler: (error: any) => {
      console.error('[tesseract] Encountered an error inside a Tesseract worker')
      console.error(error)
    }
  }
}

export async function initTesseractScheduler(options: Partial<TesseractOptions> = {}): Promise<Scheduler> {
  const { workerOptions, workerParams, numWorkers, langs } = Object.assign(options, TesseractDefaults)
  const scheduler = createScheduler()

  for (let i = 0; i < numWorkers; i++) {
    try {
      const worker = await initTesseractWorker(workerOptions, workerParams, langs)
      scheduler.addWorker(worker)
    } catch (e) {
      console.error(e)
    }
  }

  return scheduler
}

export async function initTesseractWorker(
  workerOptions: Partial<Tesseract.WorkerOptions>,
  workerParams: Partial<Tesseract.WorkerParams>,
  langs: string,
): Promise<Worker> {
  const worker = await createWorker(workerOptions)
  await worker.load()
  await worker.loadLanguage(langs)
  await worker.initialize(langs)
  await worker.setParameters(workerParams)
  return worker
}
