#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""
Test for complex UTF-8 string handling including Russian, Braille, and Emojis.
"""


def test_complex_string_integrity():
    # Russian: "Привет, мир!" (Hello, world!)
    # Braille: "⠓⠑⠇⠇⠕ ⠺⠕⠗⠇⠙" (hello world in Braille)
    # Emoji: Multi-part and variety
    complex_str = "Привет, мир! ⠓⠑⠇⠇⠕ ⠺⠕⠗⠇⠙ 🌍🚀🌟 👨‍👩‍👧‍👦 🏳️‍🌈"

    # Assertions to ensure the string is correctly represented
    assert "Привет" in complex_str
    assert "⠓⠑⠇⠇⠕" in complex_str
    assert "🌍🚀🌟" in complex_str
    assert len(complex_str) > 0
    print(f"Verified complex string: {complex_str}")


if __name__ == "__main__":
    test_complex_string_integrity()
