
package com.example

import java.util.Date

/**
 * A sample class for testing.
 */
class User(val name: String, val id: Int) {
    fun getDisplayName(): String {
        return "User: $name ($id)"
    }
}

interface Repository {
    fun findById(id: Int): User?
}

object Database {
    val url = "jdbc:mysql://localhost:3306/db"
}

typealias UserId = Int

fun main() {
    println("Hello, Kotlin!")
}
