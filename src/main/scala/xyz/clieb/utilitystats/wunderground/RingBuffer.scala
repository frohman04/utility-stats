package xyz.clieb.utilitystats.wunderground

import scala.collection.mutable

private[wunderground] class RingBuffer[T](size: Int) {
  private val buffer = mutable.IndexedSeq.fill[T](size)(null.asInstanceOf[T])
  private var curr = 0

  def add(value: T): Unit = {
    curr = (curr + 1) % buffer.length
    buffer(curr) = value
  }

  def peekTail: T = buffer(curr)
}
