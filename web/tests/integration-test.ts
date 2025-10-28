/**
 * Integration Test for Elizabeth Frontend
 *
 * This script tests the complete flow of:
 * 1. Creating a room
 * 2. Getting access token
 * 3. Sending messages
 * 4. Updating messages
 * 5. Deleting messages
 * 6. Updating room settings
 */

import { createRoom, getRoomDetails, updateRoomSettings } from '../api/roomService';
import { getAccessToken } from '../api/authService';
import { postMessage, getMessages, updateMessage, deleteMessage } from '../api/messageService';

async function runIntegrationTest() {
  console.log('=== Elizabeth Frontend Integration Test ===\n');

  try {
    // Test 1: Create a room
    const roomName = `test-room-${Date.now()}`;
    const password = 'test123';

    console.log(`1. Creating room: ${roomName} with password: ${password}`);
    const room = await createRoom(roomName, password);
    console.log('✓ Room created:', room);
    console.log('');

    // Test 2: Get access token
    console.log('2. Getting access token');
    const tokenResponse = await getAccessToken(roomName, password);
    console.log('✓ Token obtained:', tokenResponse.token.substring(0, 50) + '...');
    console.log('');

    // Test 3: Get room details
    console.log('3. Getting room details');
    const roomDetails = await getRoomDetails(roomName, tokenResponse.token);
    console.log('✓ Room details:', roomDetails);
    console.log('');

    // Test 4: Send a message
    console.log('4. Sending a message');
    const messageContent = 'Hello from integration test!';
    const message = await postMessage(roomName, messageContent, tokenResponse.token);
    console.log('✓ Message sent:', message);
    console.log('');

    // Test 5: Get all messages
    console.log('5. Getting all messages');
    const messages = await getMessages(roomName, tokenResponse.token);
    console.log('✓ Messages retrieved:', messages.length, 'messages');
    console.log('');

    // Test 6: Update the message
    console.log('6. Updating the message');
    const updatedContent = 'Updated message content!';
    const updatedMessage = await updateMessage(roomName, message.id, updatedContent, tokenResponse.token);
    console.log('✓ Message updated:', updatedMessage);
    console.log('');

    // Test 7: Update room settings
    console.log('7. Updating room settings');
    const updatedRoom = await updateRoomSettings(roomName, tokenResponse.token, {
      maxSize: 20971520, // 20MB
    });
    console.log('✓ Room settings updated:', updatedRoom);
    console.log('');

    // Test 8: Delete the message
    console.log('8. Deleting the message');
    await deleteMessage(roomName, message.id, tokenResponse.token);
    console.log('✓ Message deleted');
    console.log('');

    // Test 9: Verify message is deleted
    console.log('9. Verifying message deletion');
    const messagesAfterDelete = await getMessages(roomName, tokenResponse.token);
    console.log('✓ Messages after deletion:', messagesAfterDelete.length, 'messages');
    console.log('');

    console.log('=== All Tests Passed! ===');
    return true;
  } catch (error) {
    console.error('✗ Test failed:', error);
    return false;
  }
}

// Run the test if this file is executed directly
if (typeof window === 'undefined') {
  runIntegrationTest().then(success => {
    process.exit(success ? 0 : 1);
  });
}

export { runIntegrationTest };
