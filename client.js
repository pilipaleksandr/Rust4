const WS_URL = "ws://127.0.0.1:3000/ws";


const loginContainer = document.getElementById("loginContainer");
const chatContainer = document.getElementById("chatContainer");
const usernameInput = document.getElementById("username");
const passwordInput = document.getElementById("password");
const loginBtn = document.getElementById("loginBtn");
const messagesDiv = document.getElementById("messages");
const messageInput = document.getElementById("messageInput");
const sendMessageBtn = document.getElementById("sendMessageBtn");

let socket;
let username = "";


function connectWebSocket() {
  try {
    console.log(`Попытка подключения к WebSocket: ${WS_URL}`);
    socket = new WebSocket(WS_URL);

    socket.onopen = () => {
      console.log("Соединение с WebSocket-сервером установлено");
    };

    socket.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        console.log("Получено сообщение от сервера:", data);

        if (data.type === "auth") {
          handleAuthResponse(data);
        } else if (data.type === "message") {
          addMessageToChat(data.message);
        } else {
          console.warn("Неизвестный тип сообщения:", data);
        }
      } catch (err) {
        console.error("Ошибка обработки сообщения:", err);
      }
    };

    socket.onclose = () => {
      console.warn("Соединение с WebSocket-сервером закрыто");
      alert("Соединение с сервером потеряно. Попытка переподключения...");
      setTimeout(connectWebSocket, 3000);
    };

    socket.onerror = (error) => {
      console.error("Ошибка WebSocket:", error);
      alert("Не удалось подключиться к серверу. Проверьте подключение.");
    };
  } catch (err) {
    console.error("Ошибка инициализации WebSocket:", err);
    alert("Ошибка подключения к WebSocket. Попробуйте перезапустить сервер.");
  }
}


function handleAuthResponse(data) {
  console.log("Ответ авторизации от сервера:", data);

  if (data.status === "success") {
    username = data.username;
    loginContainer.classList.add("hidden");
    chatContainer.classList.remove("hidden");
    console.log("Авторизация успешна");
  } else {
    alert(data.message || "Ошибка авторизации");
  }
}


function addMessageToChat(message) {
  const messageElement = document.createElement("div");
  messageElement.textContent = `[${message.username}] ${message.text}`;
  messagesDiv.appendChild(messageElement);
  messagesDiv.scrollTop = messagesDiv.scrollHeight;
}


function sendMessage() {
  const text = messageInput.value.trim();
  if (text === "") return;

  if (socket.readyState === WebSocket.OPEN) {
    const message = {
      type: "message",
      username,
      text,
    };

    try {
      socket.send(JSON.stringify(message));
      messageInput.value = "";
    } catch (error) {
      console.error("Ошибка отправки сообщения:", error);
      alert("Не удалось отправить сообщение.");
    }
  } else {
    alert("Соединение с сервером отсутствует. Сообщение не отправлено.");
  }
}


loginBtn.addEventListener("click", () => {
  const username = usernameInput.value.trim();
  const password = passwordInput.value.trim();

  if (username === "" || password === "") {
    alert("Введите логин и пароль");
    return;
  }

  if (socket.readyState === WebSocket.OPEN) {
    const authMessage = {
      type: "auth",
      username,
      password,
    };

    try {
      socket.send(JSON.stringify(authMessage));
    } catch (err) {
      console.error("Ошибка отправки авторизационного запроса:", err);
      alert("Не удалось выполнить авторизацию.");
    }
  } else {
    alert("Не удалось подключиться к серверу. Попробуйте еще раз позже.");
  }
});


sendMessageBtn.addEventListener("click", sendMessage);


connectWebSocket();
